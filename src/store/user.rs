#[derive(Debug)]
pub struct User {
    age: u8,
    is_female: bool,
    height: u16,
    activity_level: u8,
}

impl User {
    pub fn new(age: u8, is_female: bool, height: u16, activity_level: u8) -> Self {
        Self {
            age,
            is_female,
            height,
            activity_level,
        }
    }
    pub fn age(&self) -> u8 {
        self.age
    }
    pub fn is_female(&self) -> bool {
        self.is_female
    }
    pub fn height(&self) -> u16 {
        self.height
    }
    pub fn activity_level(&self) -> u8 {
        self.activity_level
    }
}
