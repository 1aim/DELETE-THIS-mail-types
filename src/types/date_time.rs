use std::ops::Deref;

use chrono;

pub struct DateTime( chrono::DateTime<chrono::Utc> );

impl DateTime {
    pub fn new<TZ: chrono::TimeZone>( date_time: chrono::DateTime<TZ>) -> DateTime {
        DateTime( date_time.with_timezone( &chrono::Utc ) )
    }
}

impl Deref for DateTime {
    type Target = chrono::DateTime<chrono::Utc>;

    fn deref( &self ) -> Self::Target {
        &self.0
    }
}