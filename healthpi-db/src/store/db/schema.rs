table! {
    record_values (id) {
        id -> Binary,
        record_id -> Binary,
        value -> Double,
        value_type -> Integer,
    }
}

table! {
    records (id) {
        id -> Binary,
        timestamp -> BigInt,
        source -> Text,
    }
}

joinable!(record_values -> records (record_id));

allow_tables_to_appear_in_same_query!(
    record_values,
    records,
);
