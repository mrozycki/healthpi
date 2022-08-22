CREATE TABLE
    records (
        id BLOB NOT NULL PRIMARY KEY,
        timestamp BIGINT NOT NULL,
        source TEXT NOT NULL
    );

CREATE TABLE
    record_values (
        id BLOB NOT NULL PRIMARY KEY,
        record_id BLOB NOT NULL,
        value DOUBLE NOT NULL,
        value_type INTEGER NOT NULL,
        FOREIGN KEY(record_id) REFERENCES records(id)
    );