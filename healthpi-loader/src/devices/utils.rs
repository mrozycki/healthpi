use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

/// Create NaiveDateTime from 7 bytes, representing in order:
/// year, month, date, hours, minutes, seconds. Year is represented
/// by 2 bytes in big-endian order.
pub fn naive_date_time_from_be_bytes(bytes: &[u8]) -> NaiveDateTime {
    naive_date_time_from_bytes(bytes, false)
}

/// Create NaiveDateTime from 7 bytes, representing in order:
/// year, month, date, hours, minutes, seconds. Year is represented
/// by 2 bytes in little-endian order.
pub fn naive_date_time_from_le_bytes(bytes: &[u8]) -> NaiveDateTime {
    naive_date_time_from_bytes(bytes, true)
}

fn naive_date_time_from_bytes(bytes: &[u8], le: bool) -> NaiveDateTime {
    let year = if le {
        u16::from_le_bytes([bytes[0], bytes[1]])
    } else {
        u16::from_be_bytes([bytes[0], bytes[1]])
    };
    let date = NaiveDate::from_ymd(year as i32, bytes[2] as u32, bytes[3] as u32);
    let time = NaiveTime::from_hms(bytes[4] as u32, bytes[5] as u32, bytes[6] as u32);

    NaiveDateTime::new(date, time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naive_date_time_from_le_bytes() {
        let test_data = [
            (
                [7, 228, 2, 29, 21, 37, 5],
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2020, 2, 29),
                    NaiveTime::from_hms(21, 37, 5),
                ),
            ),
            (
                [7, 216, 10, 1, 1, 1, 1],
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2008, 10, 1),
                    NaiveTime::from_hms(1, 1, 1),
                ),
            ),
            (
                [8, 2, 12, 22, 6, 23, 10],
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2050, 12, 22),
                    NaiveTime::from_hms(6, 23, 10),
                ),
            ),
            (
                [7, 230, 8, 22, 16, 2, 19],
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2022, 8, 22),
                    NaiveTime::from_hms(16, 2, 19),
                ),
            ),
        ];

        for (i, (bytes, expected_output)) in test_data.into_iter().enumerate() {
            assert_eq!(
                naive_date_time_from_bytes(&bytes, false),
                expected_output,
                "Test case #{}",
                i
            );
        }
    }

    #[test]
    fn naive_date_time_from_be_bytes() {
        let test_data = [
            (
                [228, 7, 2, 29, 21, 37, 5],
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2020, 2, 29),
                    NaiveTime::from_hms(21, 37, 5),
                ),
            ),
            (
                [216, 7, 10, 1, 1, 1, 1],
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2008, 10, 1),
                    NaiveTime::from_hms(1, 1, 1),
                ),
            ),
            (
                [2, 8, 12, 22, 6, 23, 10],
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2050, 12, 22),
                    NaiveTime::from_hms(6, 23, 10),
                ),
            ),
            (
                [230, 7, 8, 22, 16, 2, 19],
                NaiveDateTime::new(
                    NaiveDate::from_ymd(2022, 8, 22),
                    NaiveTime::from_hms(16, 2, 19),
                ),
            ),
        ];

        for (i, (bytes, expected_output)) in test_data.into_iter().enumerate() {
            assert_eq!(
                naive_date_time_from_bytes(&bytes, true),
                expected_output,
                "Test case #{}",
                i
            );
        }
    }
}
