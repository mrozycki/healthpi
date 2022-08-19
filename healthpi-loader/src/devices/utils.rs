use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

pub fn naive_date_time_from_be_bytes(bytes: &[u8]) -> NaiveDateTime {
    naive_date_time_from_bytes(bytes, false)
}

pub fn naive_date_time_from_le_bytes(bytes: &[u8]) -> NaiveDateTime {
    naive_date_time_from_bytes(bytes, true)
}

pub fn naive_date_time_from_bytes(bytes: &[u8], le: bool) -> NaiveDateTime {
    let year = if le {
        u16::from_le_bytes([bytes[0], bytes[1]])
    } else {
        u16::from_be_bytes([bytes[0], bytes[1]])
    };
    let date = NaiveDate::from_ymd(year as i32, bytes[2] as u32, bytes[3] as u32);
    let time = NaiveTime::from_hms(bytes[4] as u32, bytes[5] as u32, bytes[6] as u32);

    NaiveDateTime::new(date, time)
}
