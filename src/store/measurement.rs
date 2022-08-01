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
    WaterPercent(f64),
    MusclePercent(f64),
    FatPercent(f64),
    Glucose(i32),
    Meal(MealIndicator),
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Record {
    timestamp: NaiveDateTime,
    values: Vec<Value>,
}

impl Record {
    pub fn with_values(timestamp: NaiveDateTime, values: Vec<Value>) -> Self {
        Self { timestamp, values }
    }

    pub fn add_value(&mut self, value: Value) {
        self.values.push(value)
    }
}
