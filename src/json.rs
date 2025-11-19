use std::collections::HashMap;

use serde_json::{Number, Value};

pub enum JsonValue {
    Null,
    Bool(bool),
    Str(String),
    Float(f64),
    PositiveInt(u64),
    NegativeInt(i64),
    List(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl From<&JsonValue> for Value {
    fn from(value: &JsonValue) -> Self {
        match value {
            JsonValue::Null => Value::Null,
            JsonValue::Bool(val) => Value::Bool(*val),
            JsonValue::Str(val) => Value::String(val.to_string()),
            JsonValue::Float(val) => Value::Number(Number::from_f64(*val).unwrap()),
            JsonValue::PositiveInt(val) => Value::Number(Number::from_u128(*val as u128).unwrap()),
            JsonValue::NegativeInt(val) => Value::Number(Number::from_i128(*val as i128).unwrap()),
            JsonValue::List(list) => {
                Value::Array(list.into_iter().map(|v| Value::from(v)).collect())
            }
            JsonValue::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.to_string(), Value::from(v)))
                    .collect(),
            ),
        }
    }
}

// Useful for the external library users

impl TryFrom<&str> for JsonValue {
    type Error = std::io::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = serde_json::from_str::<Value>(value)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        Ok(JsonValue::from(value))
    }
}
impl From<Value> for JsonValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => JsonValue::Null,
            Value::Bool(val) => JsonValue::Bool(val),
            Value::Number(val) => {
                if val.is_i64() {
                    JsonValue::NegativeInt(val.as_i64().unwrap())
                } else if val.is_u64() {
                    JsonValue::PositiveInt(val.as_u64().unwrap())
                } else if val.is_f64() {
                    JsonValue::Float(val.as_f64().unwrap())
                } else {
                    unreachable!()
                }
            }
            Value::String(val) => JsonValue::Str(val),
            Value::Array(list) => JsonValue::List(
                list.into_iter()
                    .map(|val| JsonValue::from(val))
                    .collect::<Vec<JsonValue>>(),
            ),
            Value::Object(map) => JsonValue::Object(
                map.into_iter()
                    .map(|(k, v)| (k, JsonValue::from(v)))
                    .collect(),
            ),
        }
    }
}
