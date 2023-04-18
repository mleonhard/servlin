use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TagValue {
    Str(&'static str),
    String(String),
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Float(String),
    Null,
}
impl TagValue {
    #[must_use]
    pub fn ordinal(&self) -> usize {
        match self {
            TagValue::Str(..) => 0,
            TagValue::String(..) => 1,
            TagValue::Bool(..) => 2,
            TagValue::I8(..) => 3,
            TagValue::I16(..) => 4,
            TagValue::I32(..) => 5,
            TagValue::I64(..) => 6,
            TagValue::I128(..) => 7,
            TagValue::U8(..) => 8,
            TagValue::U16(..) => 9,
            TagValue::U32(..) => 10,
            TagValue::U64(..) => 11,
            TagValue::U128(..) => 12,
            TagValue::Float(..) => 13,
            TagValue::Null => 14,
        }
    }
}
impl From<&str> for TagValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}
impl From<String> for TagValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<bool> for TagValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<i8> for TagValue {
    fn from(value: i8) -> Self {
        Self::I8(value)
    }
}
impl From<i16> for TagValue {
    fn from(value: i16) -> Self {
        Self::I16(value)
    }
}
impl From<i32> for TagValue {
    fn from(value: i32) -> Self {
        Self::I32(value)
    }
}
impl From<i64> for TagValue {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}
impl From<i128> for TagValue {
    fn from(value: i128) -> Self {
        Self::I128(value)
    }
}
impl From<u8> for TagValue {
    fn from(value: u8) -> Self {
        Self::U8(value)
    }
}
impl From<u16> for TagValue {
    fn from(value: u16) -> Self {
        Self::U16(value)
    }
}
impl From<u32> for TagValue {
    fn from(value: u32) -> Self {
        Self::U32(value)
    }
}
impl From<u64> for TagValue {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}
impl From<u128> for TagValue {
    fn from(value: u128) -> Self {
        Self::U128(value)
    }
}
impl From<f32> for TagValue {
    fn from(value: f32) -> Self {
        Self::Float(format!("{value}"))
    }
}
impl From<f64> for TagValue {
    fn from(value: f64) -> Self {
        Self::Float(format!("{value}"))
    }
}
impl<T: Into<TagValue>> From<Option<T>> for TagValue {
    fn from(value: Option<T>) -> Self {
        match value {
            None => Self::Null,
            Some(t) => t.into(),
        }
    }
}
impl Display for TagValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            TagValue::Str(x) => write!(f, "{x:?}"),
            TagValue::String(x) => write!(f, "{x:?}"),
            TagValue::Bool(x) => Display::fmt(&x, f),
            TagValue::I8(x) => Display::fmt(&x, f),
            TagValue::I16(x) => Display::fmt(&x, f),
            TagValue::I32(x) => Display::fmt(&x, f),
            TagValue::I64(x) => Display::fmt(&x, f),
            TagValue::I128(x) => Display::fmt(&x, f),
            TagValue::U8(x) => Display::fmt(&x, f),
            TagValue::U16(x) => Display::fmt(&x, f),
            TagValue::U32(x) => Display::fmt(&x, f),
            TagValue::U64(x) => Display::fmt(&x, f),
            TagValue::U128(x) => Display::fmt(&x, f),
            TagValue::Float(x) => Display::fmt(&x, f),
            TagValue::Null => write!(f, "null"),
        }
    }
}
