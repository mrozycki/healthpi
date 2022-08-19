use std::{
    env,
    error::Error,
    sync::{Arc, LockResult, Mutex, MutexGuard},
};

use diesel::{Connection as DieselConnection, SqliteConnection};
use dotenv::dotenv;

#[derive(Clone)]
pub struct Connection {
    inner: Arc<Mutex<SqliteConnection>>,
}

impl Connection {
    pub fn establish() -> Result<Self, Box<dyn Error>> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let connection = SqliteConnection::establish(&database_url)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn lock<'a>(&'a self) -> LockResult<MutexGuard<'a, SqliteConnection>> {
        self.inner.lock()
    }
}
