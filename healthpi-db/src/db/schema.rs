table! {
    record_values (record_ref, value_type) {
        record_ref -> Binary,
        value -> Double,
        value_type -> Integer,
    }
}

table! {
    records (timestamp, source) {
        timestamp -> BigInt,
        source -> Text,
        record_ref -> Binary,
    }
}

allow_tables_to_appear_in_same_query!(record_values, records,);
