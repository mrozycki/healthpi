table! {
    record_values (id) {
        id -> Integer,
        record_id -> Integer,
        value -> Double,
        value_type -> Integer,
    }
}

table! {
    records (id) {
        id -> Integer,
        timestamp -> BigInt,
        source -> Text,
    }
}

joinable!(record_values -> records (record_id));

allow_tables_to_appear_in_same_query!(
    record_values,
    records,
);
