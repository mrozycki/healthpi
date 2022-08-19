use std::error::Error;

use diesel::RunQueryDsl;

use crate::measurement::{Record, Value};

use super::{connection::Connection, schema::*};

#[derive(Insertable)]
#[diesel(table_name = records)]
pub struct NewRecord {
    timestamp: i64,
    source: String,
}

impl Into<(NewRecord, Vec<Value>)> for Record {
    fn into(self) -> (NewRecord, Vec<Value>) {
        let new_record = NewRecord {
            timestamp: self.timestamp.timestamp(),
            source: format!("{:?}", self.source),
        };
        (new_record, self.values)
    }
}

#[derive(Insertable)]
#[diesel(table_name = record_values)]
pub struct NewValue {
    record_id: i32,
    value_type: i32,
    value: f64,
}

impl NewValue {
    pub fn from_value(dto: Value, record_id: i32) -> Self {
        let (value_type, value): (usize, f64) = dto.into();
        Self {
            record_id,
            value_type: value_type as i32,
            value,
        }
    }
}

pub struct MeasurementRepository {
    connection: Connection,
}

impl MeasurementRepository {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub fn store_records(&self, records: Vec<Record>) -> Result<(), Box<dyn Error>> {
        mod record_values {
            pub use crate::db::schema::record_values::dsl::*;
        }
        mod records {
            pub use crate::db::schema::records::dsl::*;
        }

        let mut conn = self.connection.lock().map_err(|e| e.to_string())?;
        for record in records.into_iter() {
            let (new_record, values) = record.into();
            let new_record_id = diesel::insert_into(records::records)
                .values(&new_record)
                .returning(records::id)
                .get_result(&mut *conn)?;

            let new_values = values
                .into_iter()
                .map(|value| NewValue::from_value(value, new_record_id))
                .collect::<Vec<_>>();

            diesel::insert_into(record_values::record_values)
                .values(new_values)
                .execute(&mut *conn)?;
        }
        Ok(())
    }
}
