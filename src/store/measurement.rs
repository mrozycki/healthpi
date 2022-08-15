use bluez_async::MacAddress;
use chrono::NaiveDateTime;

#[derive(Debug)]
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

#[derive(Debug)]
pub enum Source {
    Device(MacAddress),
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Record {
    timestamp: NaiveDateTime,
    values: Vec<Value>,
    raw_data: Vec<u8>,
    source: Source,
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
