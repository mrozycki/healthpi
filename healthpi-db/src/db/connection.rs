use std::{env, error::Error, sync::Arc};

use dotenv::dotenv;
use sqlx::{Connection as SqlxConnection, Executor, SqliteConnection};
use tokio::sync::{Mutex, MutexGuard};

const SETUP_QUERY: &str = "PRAGMA mmap_size = 30000000000;
PRAGMA cache_size = -1000;
PRAGMA page_size = 4096;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;";

#[derive(Clone)]
pub struct Connection {
    inner: Arc<Mutex<SqliteConnection>>,
}

impl Connection {
    pub async fn establish() -> Result<Self, Box<dyn Error>> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let mut connection = SqliteConnection::connect(&database_url).await?;

        connection.execute(SETUP_QUERY).await?;

        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
        })
    }

    pub async fn lock(&self) -> MutexGuard<'_, SqliteConnection> {
        self.inner.lock().await
    }
}
