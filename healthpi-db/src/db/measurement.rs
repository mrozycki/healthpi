use std::{
    error::Error,
    hash::{Hash, Hasher},
};

use diesel::RunQueryDsl;
use log::debug;
use rustc_hash::FxHasher;

use crate::measurement::{Record, Value};

use super::{connection::Connection, schema::*};

#[derive(Insertable)]
#[table_name = "records"]
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

#[derive(Insertable)]
#[table_name = "record_values"]
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
pub trait MeasurementRepository: Send + Sync {
    fn store_records(&self, records: Vec<Record>) -> Result<(), Box<dyn Error>>;
}

pub struct MeasurementRepositoryImpl {
    connection: Connection,
}

impl MeasurementRepositoryImpl {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }
}

impl MeasurementRepository for MeasurementRepositoryImpl {
    fn store_records(&self, records: Vec<Record>) -> Result<(), Box<dyn Error>> {
        mod record_values {
            pub use crate::db::schema::record_values::dsl::*;
        }
        mod records {
            pub use crate::db::schema::records::dsl::*;
        }

        debug!("Converting records");
        let (new_records, new_values_vecs): (Vec<NewRecord>, Vec<Vec<NewValue>>) =
            records.into_iter().map(Into::into).unzip();
        let new_values: Vec<NewValue> = new_values_vecs.into_iter().flatten().collect();

        let mut conn = self.connection.lock().map_err(|e| e.to_string())?;
        debug!("Storing records");
        diesel::replace_into(records::records)
            .values(new_records)
            .execute(&mut *conn)?;

        debug!("Storing values");
        diesel::replace_into(record_values::record_values)
            .values(new_values)
            .execute(&mut *conn)?;
        Ok(())
    }
}
