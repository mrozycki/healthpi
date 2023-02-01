use std::{
    error::Error,
    hash::{Hash, Hasher},
};

use async_trait::async_trait;
use chrono::NaiveDateTime;
use itertools::Itertools;
use log::debug;
use rustc_hash::FxHasher;
use sqlx::{query, QueryBuilder};

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
    async fn fetch_records(&self) -> Result<Vec<Record>, Box<dyn Error>>;
}

#[derive(Clone)]
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
                    .push_bind(ron::to_string(&record.source).unwrap())
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

    async fn fetch_records(&self) -> Result<Vec<Record>, Box<dyn Error>> {
        let mut conn = self.connection.lock().await;
        query!(
            r#"SELECT timestamp, source, value, value_type
            FROM records, record_values 
            WHERE records.record_ref = record_values.record_ref
            ORDER BY timestamp DESC, source"#
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .group_by(|s| (s.timestamp, s.source.clone()))
        .into_iter()
        .map(
            |((timestamp, source), values)| -> Result<_, Box<dyn Error>> {
                Ok(Record::new(
                    NaiveDateTime::from_timestamp_opt(timestamp, 0)
                        .ok_or_else(|| format!("Invalid timestamp: {}", timestamp))?,
                    values
                        .into_iter()
                        .map(|s| (s.value_type as usize, s.value).try_into().unwrap())
                        .collect(),
                    Vec::new(),
                    ron::from_str(&source)?,
                ))
            },
        )
        .collect()
    }
}
