use std::{
    env,
    error::Error,
    sync::{Arc, LockResult, Mutex, MutexGuard},
};

use diesel::{connection::SimpleConnection, Connection as DieselConnection, SqliteConnection};
use dotenv::dotenv;

const SETUP_QUERY: &'static str = "PRAGMA mmap_size = 30000000000;
PRAGMA cache_size = -1000;
PRAGMA page_size = 4096;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;";

#[derive(Clone)]
pub struct Connection {
    inner: Arc<Mutex<SqliteConnection>>,
}

impl Connection {
    pub fn establish() -> Result<Self, Box<dyn Error>> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let connection = SqliteConnection::establish(&database_url)?;

        connection.batch_execute(SETUP_QUERY)?;

        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn lock<'a>(&'a self) -> LockResult<MutexGuard<'a, SqliteConnection>> {
        self.inner.lock()
    }
}
