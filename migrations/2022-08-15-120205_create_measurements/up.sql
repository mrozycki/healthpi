CREATE TABLE
    records (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        timestamp BIGINT NOT NULL,
        source TEXT NOT NULL
    );

CREATE TABLE
    record_values (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        record_id INTEGER NOT NULL,
        value DOUBLE NOT NULL,
        value_type INTEGER NOT NULL,
        FOREIGN KEY(record_id) REFERENCES records(id)
    );