CREATE TABLE
    records (
        timestamp BIGINT NOT NULL,
        source TEXT NOT NULL,
        record_ref BLOB NOT NULL UNIQUE,
        PRIMARY KEY(timestamp, source)
    );

CREATE TABLE
    record_values (
        record_ref BLOB NOT NULL,
        value DOUBLE NOT NULL,
        value_type INTEGER NOT NULL,
        PRIMARY KEY(record_ref, value_type),
        FOREIGN KEY(record_ref) REFERENCES records(record_ref)
    );