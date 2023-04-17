use crate::log::tag_value::TagValue;
use std::fmt::{Debug, Display, Formatter};

pub fn tag(name: &'static str, value: impl Into<TagValue>) -> Tag {
    Tag::new(name, value)
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Tag {
    pub name: &'static str,
    pub value: TagValue,
}
impl Tag {
    pub fn new(name: &'static str, value: impl Into<TagValue>) -> Self {
        Self {
            name,
            value: value.into(),
        }
    }
}
impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}: {}", self.name, self.value)
    }
}
impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Tag{{{:?}:{:?}}}", self.name, self.value)
    }
}
