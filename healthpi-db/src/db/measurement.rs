use std::{
    error::Error,
    hash::{Hash, Hasher},
};

use async_trait::async_trait;
use log::debug;
use rustc_hash::FxHasher;
use sqlx::QueryBuilder;

use crate::measurement::{Record, Value};

use super::connection::Connection;

pub struct NewRecord {
    record_ref: Vec<u8>,
    timestamp: i64,
    source: String,
}

impl Into<(NewRecord, Vec<NewValue>)> for Record {
    fn into(self) -> (NewRecord, Vec<NewValue>) {
        let mut hasher = FxHasher::default();
        self.timestamp.hash(&mut hasher);
        self.source.hash(&mut hasher);
        let record_ref = hasher.finish().to_le_bytes().to_vec();

        let new_values = self
            .values
            .into_iter()
            .map(|value| NewValue::from_value(value, record_ref.clone()))
            .collect();
        let new_record = NewRecord {
            record_ref,
            timestamp: self.timestamp.timestamp(),
            source: format!("{:?}", self.source),
        };

        (new_record, new_values)
    }
}

pub struct NewValue {
    record_ref: Vec<u8>,
    value_type: i32,
    value: f64,
}

impl NewValue {
    pub fn from_value(dto: Value, record_ref: Vec<u8>) -> Self {
        let (value_type, value): (usize, f64) = dto.into();
        Self {
            record_ref,
            value_type: value_type as i32,
            value,
        }
    }
}

#[mockall::automock]
#[async_trait]
pub trait MeasurementRepository: Send + Sync {
    async fn store_records(&self, records: Vec<Record>) -> Result<(), Box<dyn Error>>;
}

pub struct MeasurementRepositoryImpl {
    connection: Connection,
}

impl MeasurementRepositoryImpl {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl MeasurementRepository for MeasurementRepositoryImpl {
    async fn store_records(&self, records: Vec<Record>) -> Result<(), Box<dyn Error>> {
        debug!("Converting records");
        let (new_records, new_values_vecs): (Vec<NewRecord>, Vec<Vec<NewValue>>) =
            records.into_iter().map(Into::into).unzip();
        let new_values: Vec<NewValue> = new_values_vecs.into_iter().flatten().collect();

        let mut conn = self.connection.lock().await;

        debug!("Storing records");
        QueryBuilder::new("INSERT INTO records(timestamp, source, record_ref) ")
            .push_values(new_records, |mut b, record| {
                b.push_bind(record.timestamp)
                    .push_bind(record.source)
                    .push_bind(record.record_ref);
            })
            .push(" ON CONFLICT DO NOTHING ")
            .build()
            .execute(&mut *conn)
            .await?;

        debug!("Storing values");
        QueryBuilder::new("INSERT INTO record_values(record_ref, value, value_type) ")
            .push_values(new_values, |mut b, value| {
                b.push_bind(value.record_ref)
                    .push_bind(value.value)
                    .push_bind(value.value_type);
            })
            .push(" ON CONFLICT DO UPDATE SET value=excluded.value ")
            .build()
            .execute(&mut *conn)
            .await?;

        Ok(())
    }
}
