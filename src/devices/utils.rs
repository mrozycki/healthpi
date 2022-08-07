use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

pub fn naive_date_time_from_bytes(bytes: &[u8]) -> NaiveDateTime {
    let year = u16::from_be_bytes([bytes[0], bytes[1]]);
    let date = NaiveDate::from_ymd(year as i32, bytes[2] as u32, bytes[3] as u32);
    let time = NaiveTime::from_hms(bytes[4] as u32, bytes[5] as u32, bytes[6] as u32);

    NaiveDateTime::new(date, time)
}
