use std::collections::HashMap;

use serde_json::Value;

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
            (Body::Json(json), BodyMatcher::Json(matcher)) => matcher.score(json),
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
            TextMatcher::Regex(_) => todo!(),
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

// Numbers

pub enum IntMatcher {
    Eq(i128),
    LessThan(i128),
    GreaterThan(i128),
}

impl IntMatcher {
    pub fn score(&self, value: &i128) -> u16 {
        match self {
            IntMatcher::Eq(m) if m.eq(value) => 2,
            IntMatcher::LessThan(m) if m.gt(value) => 1,
            IntMatcher::GreaterThan(m) if m.lt(value) => 1,
            _ => 0,
        }
    }
}

pub enum FloatMatcher {
    Eq(f64),
    LessThan(f64),
    GreaterThan(f64),
}

impl FloatMatcher {
    pub fn score(&self, value: &f64) -> u16 {
        match self {
            FloatMatcher::Eq(m) if m.eq(value) => 2,
            FloatMatcher::LessThan(m) if m.gt(value) => 1,
            FloatMatcher::GreaterThan(m) if m.lt(value) => 1,
            _ => 0,
        }
    }
}

// Json

pub enum JsonMatcher {
    Null,
    Bool(bool),
    Str(TextMatcher),
    Int(IntMatcher),
    Float(FloatMatcher),
    List(Vec<JsonMatcher>),
    Object(HashMap<String, JsonMatcher>),
}

impl JsonMatcher {
    pub fn score(&self, value: &JsonValue) -> u16 {
        match (value, self) {
            (JsonValue::Null, JsonMatcher::Null) => 1,
            (JsonValue::Bool(v), JsonMatcher::Bool(matcher)) if matcher.eq(v) => 1,
            (JsonValue::Str(v), JsonMatcher::Str(matcher)) => matcher.score(v),
            (JsonValue::Float(v), JsonMatcher::Float(matcher)) => matcher.score(v),
            (JsonValue::Int(v), JsonMatcher::Int(matcher)) => matcher.score(v),
            (JsonValue::List(list), JsonMatcher::List(matchers)) => {
                if matchers.len() != list.len() {
                    return 0;
                }

                let mut total_score: u16 = 0;
                for (m, item) in matchers.iter().zip(list.iter()) {
                    let score = m.score(item);
                    if score == 0 {
                        return 0;
                    }
                    total_score += score;
                }

                total_score
            }

            (JsonValue::Object(map), JsonMatcher::Object(matchers)) => {
                let mut total_score = 0;

                for (k, matcher) in matchers {
                    if let Some(object) = map.get(k) {
                        let score = matcher.score(object);
                        if score == 0 {
                            return 0;
                        }
                        total_score += score;
                    } else {
                        return 0;
                    }
                }

                total_score
            }
            (_, _) => 0,
        }
    }
}

impl From<JsonValue> for JsonMatcher {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => JsonMatcher::Null,
            JsonValue::Bool(bool) => JsonMatcher::Bool(bool),
            JsonValue::Str(str) => JsonMatcher::Str(TextMatcher::Eq(str)),
            JsonValue::Float(n) => JsonMatcher::Float(FloatMatcher::Eq(n)),
            JsonValue::Int(i) => JsonMatcher::Int(IntMatcher::Eq(i)),
            JsonValue::List(list) => JsonMatcher::List(
                list.into_iter()
                    .map(JsonMatcher::from)
                    .collect::<Vec<JsonMatcher>>(),
            ),
            JsonValue::Object(map) => JsonMatcher::Object(
                map.into_iter()
                    .map(|(k, v)| (k, JsonMatcher::from(v)))
                    .collect(),
            ),
        }
    }
}

impl From<Value> for JsonMatcher {
    fn from(value: Value) -> Self {
        JsonMatcher::from(JsonValue::from(value))
    }
}

impl TryFrom<&str> for JsonMatcher {
    type Error = std::io::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = serde_json::from_str::<Value>(value)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

        Ok(JsonMatcher::from(JsonValue::from(value)))
    }
}
