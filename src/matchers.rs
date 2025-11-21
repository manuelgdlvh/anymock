use std::collections::HashMap;

use crate::json::JsonValue;

pub enum Body {
    Json(JsonValue),
    Binary(Vec<u8>),
    PlainText(String),
}

pub enum BodyMatcher {
    Json(JsonMatcher),
    Binary(BinaryMatcher),
    PlainText(TextMatcher),
}

impl BodyMatcher {
    pub fn matches(&self, body: &Body) -> u16 {
        match (body, self) {
            (Body::Json(json), BodyMatcher::Json(matcher)) => 0,
            (Body::PlainText(part), BodyMatcher::PlainText(matcher)) => matcher.score(part),
            (Body::Binary(part), BodyMatcher::Binary(matcher)) => matcher.score(part),
            _ => 0,
        }
    }
}

// Text

pub enum TextMatcher {
    Regex(String),
    Eq(String),
    Contains(String),
}

impl TextMatcher {
    pub fn score(&self, value: &str) -> u16 {
        match self {
            TextMatcher::Regex(_) => 0,
            TextMatcher::Eq(part) if value.eq(part) => 2,
            TextMatcher::Contains(part) if value.contains(part) => 1,
            _ => 0,
        }
    }
}

pub fn text_eq(text: impl Into<String>) -> TextMatcher {
    TextMatcher::Eq(text.into())
}

pub fn text_contains(text: impl Into<String>) -> TextMatcher {
    TextMatcher::Contains(text.into())
}

// Binary

pub enum BinaryMatcher {
    Eq(Vec<u8>),
    Contains(Vec<u8>),
}

impl BinaryMatcher {
    pub fn score(&self, value: &[u8]) -> u16 {
        match self {
            BinaryMatcher::Eq(part) if value.eq(part) => 2,
            BinaryMatcher::Contains(part)
                if value.windows(part.len()).any(|subpart| subpart == part) =>
            {
                1
            }
            _ => 0,
        }
    }
}

// Json

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
