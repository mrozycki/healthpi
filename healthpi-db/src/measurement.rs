use chrono::NaiveDateTime;
use num_derive::FromPrimitive;

use super::device::MacAddress;

#[derive(Debug, FromPrimitive)]
pub enum MealIndicator {
    NoIndication,
    NoMeal,
    BeforeMeal,
    AfterMeal,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Value {
    Weight(f64),
    BodyMassIndex(f64),
    BasalMetabolicRate(f64),
    WaterPercent(f64),
    MusclePercent(f64),
    FatPercent(f64),
    Glucose(i32),
    Meal(MealIndicator),
    BloodPressureSystolic(i32),
    BloodPressureDiastolic(i32),
    HeartRate(i32),
}

impl Into<(usize, f64)> for Value {
    fn into(self) -> (usize, f64) {
        match self {
            Value::Weight(x) => (0, x),
            Value::BodyMassIndex(x) => (1, x),
            Value::BasalMetabolicRate(x) => (2, x),
            Value::WaterPercent(x) => (3, x),
            Value::MusclePercent(x) => (4, x),
            Value::FatPercent(x) => (5, x),
            Value::Glucose(x) => (6, x as f64),
            Value::Meal(x) => (7, x as u8 as f64),
            Value::BloodPressureSystolic(x) => (8, x as f64),
            Value::BloodPressureDiastolic(x) => (9, x as f64),
            Value::HeartRate(x) => (10, x as f64),
        }
    }
}

impl TryFrom<(usize, f64)> for Value {
    type Error = &'static str;

    fn try_from((index, x): (usize, f64)) -> Result<Self, Self::Error> {
        match index {
            0 => Ok(Value::Weight(x)),
            1 => Ok(Value::BodyMassIndex(x)),
            2 => Ok(Value::BasalMetabolicRate(x)),
            3 => Ok(Value::WaterPercent(x)),
            4 => Ok(Value::MusclePercent(x)),
            5 => Ok(Value::FatPercent(x)),
            6 => Ok(Value::Glucose(x as i32)),
            7 => num::FromPrimitive::from_f64(x)
                .map(|v| Value::Meal(v))
                .ok_or("Invalid meal indicator"),
            8 => Ok(Value::BloodPressureSystolic(x as i32)),
            9 => Ok(Value::BloodPressureDiastolic(x as i32)),
            10 => Ok(Value::HeartRate(x as i32)),
            _ => Err("Invalid type"),
        }
    }
}

#[derive(Debug, Hash)]
pub enum Source {
    Device(MacAddress),
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Record {
    pub timestamp: NaiveDateTime,
    pub values: Vec<Value>,
    pub raw_data: Vec<u8>,
    pub source: Source,
}

impl Record {
    pub fn new(
        timestamp: NaiveDateTime,
        values: Vec<Value>,
        raw_data: Vec<u8>,
        source: Source,
    ) -> Self {
        Self {
            timestamp,
            values,
            raw_data,
            source,
        }
    }

    pub fn add_value(&mut self, value: Value) {
        self.values.push(value)
    }
}
