use chrono::{Date, DateTime, Utc};
use serde::{self, Deserialize, Deserializer, Serializer};

pub fn serialize<S>(date: &Date<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = date.and_hms(0, 0, 0).to_rfc3339();
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Date<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    DateTime::parse_from_rfc3339(&s)
        .map(|date_time| date_time.date().with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}
