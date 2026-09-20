//! Exact, nonempty half-open source intervals. No calendar or host clock semantics.
use serde::{Deserialize, Deserializer, Serialize, de};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Interval {
    start: i64,
    end: Option<i64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalError {
    InvalidBounds,
    EmptyIntersection,
    UnboundedEnd,
}
impl IntervalError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidBounds => "E_INTERVAL_BOUNDS",
            Self::EmptyIntersection => "E_INTERVAL_EMPTY",
            Self::UnboundedEnd => "E_INTERVAL_UNBOUNDED",
        }
    }
}
impl fmt::Display for IntervalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidBounds => "Interval start must precede its finite end",
            Self::EmptyIntersection => "Intervals have no nonempty intersection",
            Self::UnboundedEnd => "Interval has no finite end",
        })
    }
}
impl std::error::Error for IntervalError {}
impl Interval {
    pub fn new(start: i64, end: Option<i64>) -> Result<Self, IntervalError> {
        if end.is_some_and(|end| start >= end) {
            return Err(IntervalError::InvalidBounds);
        }
        Ok(Self { start, end })
    }
    pub fn start(self) -> i64 {
        self.start
    }
    pub fn end(self) -> Option<i64> {
        self.end
    }
    pub fn finite_end(self) -> Result<i64, IntervalError> {
        self.end.ok_or(IntervalError::UnboundedEnd)
    }
    pub fn contains(self, instant: i64) -> bool {
        self.start <= instant && self.end.is_none_or(|end| instant < end)
    }
    /// Strict temporal precedence; touching bounds are `meets`, not `before`.
    pub fn before(self, other: Self) -> bool {
        self.end.is_some_and(|end| end < other.start)
    }
    pub fn meets(self, other: Self) -> bool {
        self.end == Some(other.start)
    }
    /// Nonempty intersection, including containment/equality; symmetric.
    pub fn overlaps(self, other: Self) -> bool {
        self.end.is_none_or(|end| other.start < end) && other.end.is_none_or(|end| self.start < end)
    }
    /// Set inclusion, including equality.
    pub fn within(self, other: Self) -> bool {
        other.start <= self.start
            && match (self.end, other.end) {
                (_, None) => true,
                (Some(end), Some(outer)) => end <= outer,
                (None, Some(_)) => false,
            }
    }
    pub fn intersection(self, other: Self) -> Result<Self, IntervalError> {
        let end = match (self.end, other.end) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, None) | (None, a) => a,
        };
        Self::new(self.start.max(other.start), end).map_err(|_| IntervalError::EmptyIntersection)
    }
}
impl<'de> Deserialize<'de> for Interval {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Start,
            End,
        }
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Interval;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a nonempty half-open interval with start and end fields")
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Interval, A::Error> {
                let mut start = None;
                let mut end = None;
                while let Some(field) = map.next_key()? {
                    match field {
                        Field::Start => {
                            if start.is_some() {
                                return Err(de::Error::duplicate_field("start"));
                            }
                            start = Some(map.next_value::<i64>()?);
                        }
                        Field::End => {
                            if end.is_some() {
                                return Err(de::Error::duplicate_field("end"));
                            }
                            end = Some(map.next_value::<Option<i64>>()?);
                        }
                    }
                }
                Interval::new(
                    start.ok_or_else(|| de::Error::missing_field("start"))?,
                    end.ok_or_else(|| de::Error::missing_field("end"))?,
                )
                .map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}
