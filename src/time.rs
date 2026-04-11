use std::{fmt, ops::Deref};

use chrono::{DateTime, SecondsFormat, TimeZone, Timelike, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    pub fn from_datetime(value: DateTime<Utc>) -> Self {
        Self(value.with_nanosecond(0).unwrap_or(value))
    }

    pub fn parse(value: &str) -> Option<Self> {
        chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|dt| Self::from_datetime(dt.with_timezone(&Utc)))
    }

    pub fn as_datetime(&self) -> DateTime<Utc> {
        self.0.to_owned()
    }

    pub fn epoch() -> Self {
        Self(Utc.timestamp_opt(0, 0).single().expect("epoch timestamp"))
    }

    pub fn iso8601(&self) -> String {
        self.0.to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    pub fn rfc2822(&self) -> String {
        self.0.to_rfc2822()
    }
}

impl Deref for Timestamp {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.iso8601())
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.iso8601())
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).ok_or_else(|| serde::de::Error::custom("invalid timestamp"))
    }
}
