use std::collections::HashMap;

use crate::Body;

pub enum BodyMatcher {
    Json(JsonMatcher),
    Binary(BinaryMatcher),
    PlainText(TextMatcher),
}

// Ensure most restrictive stubs take precedence
impl BodyMatcher {
    pub fn matches(&self, body: &Body) -> Option<u16> {
        match (body, self) {
            (Body::Json(json), BodyMatcher::Json(matcher)) => None,
            (Body::Binary(binary), BodyMatcher::Binary(matcher))
                if matcher.matches(binary.as_slice()) =>
            {
                Some(1)
            }
            (Body::PlainText(text), BodyMatcher::PlainText(matcher))
                if matcher.matches(text.as_str()) =>
            {
                Some(1)
            }
            _ => None,
        }
    }
}

pub enum JsonMatcher {
    Bool(bool),
    Str(TextMatcher),
    Int(IntegerMatcher),
    Float(FloatMatcher),
    List(Vec<JsonMatcher>),
    Object(HashMap<String, JsonMatcher>),
}

pub enum IntegerMatcher {
    Eq(i128),
    LessThan(i128),
    GreaterThan(i128),
}

pub enum FloatMatcher {
    Eq(f64),
    LessThan(f64),
    GreaterThan(f64),
}

pub enum BinaryMatcher {
    Eq(Vec<u8>),
    Contains(Vec<u8>),
}

impl BinaryMatcher {
    pub fn matches(&self, value: &[u8]) -> bool {
        match self {
            BinaryMatcher::Eq(part) => value.eq(part),
            BinaryMatcher::Contains(part) => {
                value.windows(part.len()).any(|subpart| subpart == part)
            }
        }
    }
}

pub enum TextMatcher {
    Regex(String),
    Eq(String),
    Contains(String),
}

impl TextMatcher {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            TextMatcher::Regex(_) => false,
            TextMatcher::Eq(part) => value.eq(part),
            TextMatcher::Contains(part) => value.contains(part),
        }
    }
}
