use async_trait::async_trait;
use healthpi_model::measurement::{Record, ValueType};
use itertools::Itertools;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("server unreachable")]
    CommunicationError,
    #[error("internal server error")]
    InternalServerError,
    #[error("invalid request")]
    RequestError,
    #[error("incorrect server response")]
    ResponseError,
}

type Result<T> = std::result::Result<T, Error>;

#[mockall::automock]
#[async_trait]
pub trait Client: Send + Sync {
    async fn get_records(&self) -> Result<Vec<Record>>;
    async fn get_records_with_value_types(&self, types: &[ValueType]) -> Result<Vec<Record>>;
    async fn post_records(&self, records: &[Record]) -> Result<()>;
}

pub struct ClientImpl {
    url: String,
    client: reqwest::Client,
}

impl ClientImpl {
    fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }
}

pub fn create(url: String) -> impl Client {
    ClientImpl::new(url)
}

#[async_trait]
impl Client for ClientImpl {
    async fn get_records(&self) -> Result<Vec<Record>> {
        self.client
            .get(&self.url)
            .send()
            .await
            .map_err(|_| Error::CommunicationError)
            .and_then(|resp| {
                if resp.status().is_client_error() {
                    Err(Error::RequestError)
                } else if resp.status().is_server_error() {
                    Err(Error::InternalServerError)
                } else {
                    Ok(resp)
                }
            })?
            .json()
            .await
            .map_err(|_| Error::ResponseError)
    }

    async fn get_records_with_value_types(&self, types: &[ValueType]) -> Result<Vec<Record>> {
        self.client
            .get(&self.url)
            .query(&[
                "select",
                &types.iter().map(|t| format!("{:?}", t)).join(","),
            ])
            .send()
            .await
            .map_err(|_| Error::CommunicationError)
            .and_then(|resp| {
                if resp.status().is_client_error() {
                    Err(Error::RequestError)
                } else if resp.status().is_server_error() {
                    Err(Error::InternalServerError)
                } else {
                    Ok(resp)
                }
            })?
            .json()
            .await
            .map_err(|_| Error::ResponseError)
    }

    async fn post_records(&self, records: &[Record]) -> Result<()> {
        self.client
            .post(&self.url)
            .json(&records)
            .send()
            .await
            .map_err(|_| Error::CommunicationError)
            .and_then(|resp| {
                if resp.status().is_client_error() {
                    Err(Error::RequestError)
                } else if resp.status().is_server_error() {
                    Err(Error::InternalServerError)
                } else {
                    Ok(resp)
                }
            })?
            .json()
            .await
            .map_err(|_| Error::ResponseError)
    }
}
