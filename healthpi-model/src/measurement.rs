use chrono::NaiveDateTime;
use num::FromPrimitive;
use num_derive::FromPrimitive;

use crate::device::DeviceId;

#[derive(Copy, Clone, Debug, FromPrimitive, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, strum::EnumString))]
pub enum ValueType {
    Weight,
    BodyMassIndex,
    BasalMetabolicRate,
    WaterPercent,
    MusclePercent,
    FatPercent,
    Glucose,
    Meal,
    BloodPressureSystolic,
    BloodPressureDiastolic,
    HeartRate,
}

#[derive(Debug, FromPrimitive, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MealIndicator {
    NoIndication,
    NoMeal,
    BeforeMeal,
    AfterMeal,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
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

impl From<Value> for (usize, f64) {
    fn from(val: Value) -> Self {
        match val {
            Value::Weight(x) => (ValueType::Weight as usize, x),
            Value::BodyMassIndex(x) => (ValueType::BodyMassIndex as usize, x),
            Value::BasalMetabolicRate(x) => (ValueType::BasalMetabolicRate as usize, x),
            Value::WaterPercent(x) => (ValueType::WaterPercent as usize, x),
            Value::MusclePercent(x) => (ValueType::MusclePercent as usize, x),
            Value::FatPercent(x) => (ValueType::FatPercent as usize, x),
            Value::Glucose(x) => (ValueType::Glucose as usize, x as f64),
            Value::Meal(x) => (ValueType::Meal as usize, x as u8 as f64),
            Value::BloodPressureSystolic(x) => {
                (ValueType::BloodPressureSystolic as usize, x as f64)
            }
            Value::BloodPressureDiastolic(x) => {
                (ValueType::BloodPressureDiastolic as usize, x as f64)
            }
            Value::HeartRate(x) => (ValueType::HeartRate as usize, x as f64),
        }
    }
}

impl TryFrom<(usize, f64)> for Value {
    type Error = &'static str;

    fn try_from((index, x): (usize, f64)) -> Result<Self, Self::Error> {
        match ValueType::from_usize(index) {
            Some(ValueType::Weight) => Ok(Value::Weight(x)),
            Some(ValueType::BodyMassIndex) => Ok(Value::BodyMassIndex(x)),
            Some(ValueType::BasalMetabolicRate) => Ok(Value::BasalMetabolicRate(x)),
            Some(ValueType::WaterPercent) => Ok(Value::WaterPercent(x)),
            Some(ValueType::MusclePercent) => Ok(Value::MusclePercent(x)),
            Some(ValueType::FatPercent) => Ok(Value::FatPercent(x)),
            Some(ValueType::Glucose) => Ok(Value::Glucose(x as i32)),
            Some(ValueType::Meal) => num::FromPrimitive::from_f64(x)
                .map(Value::Meal)
                .ok_or("Invalid meal indicator"),
            Some(ValueType::BloodPressureSystolic) => Ok(Value::BloodPressureSystolic(x as i32)),
            Some(ValueType::BloodPressureDiastolic) => Ok(Value::BloodPressureDiastolic(x as i32)),
            Some(ValueType::HeartRate) => Ok(Value::HeartRate(x as i32)),
            None => Err("Invalid value type"),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Source {
    Device(DeviceId),
    Unknown(String),
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde_with::serde_as)]
pub struct Record {
    pub timestamp: NaiveDateTime,
    #[cfg_attr(feature = "serde", serde_as(as = "EnumMap"))]
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
