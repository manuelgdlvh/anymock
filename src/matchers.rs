use std::collections::HashMap;

use regex::{Error, Regex};
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
    pub fn score(&self, body: Option<&Body>) -> u16 {
        match (body, self) {
            (Some(Body::Json(json)), BodyMatcher::Json(matcher)) => matcher.score(Some(json)),
            (Some(Body::PlainText(part)), BodyMatcher::PlainText(matcher)) => {
                matcher.score(Some(part))
            }
            (Some(Body::Binary(part)), BodyMatcher::Binary(matcher)) => matcher.score(Some(part)),
            _ => 0,
        }
    }
}

pub trait MatcherFn<T>: Send + Sync {
    fn score(&self, value: Option<&T>) -> u16;
}

impl<T, F> MatcherFn<T> for F
where
    F: Fn(Option<&T>) -> u16 + Send + Sync,
{
    fn score(&self, value: Option<&T>) -> u16 {
        self(value)
    }
}

// Text

pub enum TextMatcher {
    Fn(Box<dyn MatcherFn<String>>),
    Eq(String),
    Regex(Regex),
    Contains(String),
    NotContains(String),
    LenEq(usize),
    LenGreaterThan(usize),
    LenLessThan(usize),
    Any,
    None,
}

impl TextMatcher {
    pub fn score(&self, value: Option<&String>) -> u16 {
        match (self, value) {
            (TextMatcher::Eq(part), Some(v)) if v == part => 8,
            (TextMatcher::Regex(regex), Some(v)) if regex.is_match(v) => 7,
            (TextMatcher::Contains(part), Some(v)) if v.contains(part) => 6,
            (TextMatcher::NotContains(part), Some(v)) if !v.contains(part) => 5,
            (TextMatcher::LenEq(len), Some(v)) if v.len() == *len => 4,
            (TextMatcher::LenGreaterThan(len), Some(v)) if v.len() > *len => 3,
            (TextMatcher::LenLessThan(len), Some(v)) if v.len() < *len => 3,
            (TextMatcher::None, None) => 2,
            (TextMatcher::Any, Some(_)) => 1,
            (TextMatcher::Fn(matcher_fn), v) => matcher_fn.score(v),
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

pub fn text_not_contains(text: impl Into<String>) -> TextMatcher {
    TextMatcher::NotContains(text.into())
}

pub fn text_regex<'a>(regex: impl Into<&'a str>) -> Result<TextMatcher, Error> {
    Ok(TextMatcher::Regex(Regex::new(regex.into())?))
}

pub fn text_len_eq(len: usize) -> TextMatcher {
    TextMatcher::LenEq(len)
}

pub fn text_len_gt(len: usize) -> TextMatcher {
    TextMatcher::LenGreaterThan(len)
}

pub fn text_len_lt(len: usize) -> TextMatcher {
    TextMatcher::LenLessThan(len)
}
pub fn text_any() -> TextMatcher {
    TextMatcher::Any
}

pub fn text_none() -> TextMatcher {
    TextMatcher::None
}

pub fn text_fn(matcher: impl MatcherFn<String> + 'static) -> TextMatcher {
    TextMatcher::Fn(Box::new(matcher))
}

// Binary

pub enum BinaryMatcher {
    Fn(Box<dyn MatcherFn<Vec<u8>>>),
    Eq(Vec<u8>),
    Contains(Vec<u8>),
    Any,
    None,
}

impl BinaryMatcher {
    pub fn score(&self, value: Option<&Vec<u8>>) -> u16 {
        match (self, value) {
            (BinaryMatcher::Eq(part), Some(v)) if v == part => 4,
            (BinaryMatcher::Contains(part), Some(v))
                if part.len() <= v.len() && v.windows(part.len()).any(|sub| sub == part) =>
            {
                3
            }
            (BinaryMatcher::None, None) => 2,
            (BinaryMatcher::Any, Some(_)) => 1,
            (BinaryMatcher::Fn(matcher), v) => matcher.score(v),
            _ => 0,
        }
    }
}

pub fn binary_eq(buff: impl Into<Vec<u8>>) -> BinaryMatcher {
    BinaryMatcher::Eq(buff.into())
}

pub fn binary_contains(buff: impl Into<Vec<u8>>) -> BinaryMatcher {
    BinaryMatcher::Contains(buff.into())
}

pub fn binary_any() -> BinaryMatcher {
    BinaryMatcher::Any
}

pub fn binary_none() -> BinaryMatcher {
    BinaryMatcher::None
}

pub fn binary_fn(matcher: impl MatcherFn<Vec<u8>> + 'static) -> BinaryMatcher {
    BinaryMatcher::Fn(Box::new(matcher))
}

// Numbers

pub enum IntMatcher {
    Fn(Box<dyn MatcherFn<i128>>),
    Eq(i128),
    LessThan(i128),
    GreaterThan(i128),
    Any,
    None,
}

impl IntMatcher {
    pub fn score(&self, value: Option<&i128>) -> u16 {
        match (self, value) {
            (IntMatcher::Eq(m), Some(v)) if v == m => 4,
            (IntMatcher::LessThan(m), Some(v)) if v < m => 3,
            (IntMatcher::GreaterThan(m), Some(v)) if v > m => 3,
            (IntMatcher::None, None) => 2,
            (IntMatcher::Any, Some(_)) => 1,
            (IntMatcher::Fn(matcher), v) => matcher.score(v),
            _ => 0,
        }
    }
}

pub fn int_eq(num: impl Into<i128>) -> IntMatcher {
    IntMatcher::Eq(num.into())
}

pub fn int_lt(num: impl Into<i128>) -> IntMatcher {
    IntMatcher::LessThan(num.into())
}

pub fn int_gt(num: impl Into<i128>) -> IntMatcher {
    IntMatcher::GreaterThan(num.into())
}

pub fn int_any() -> IntMatcher {
    IntMatcher::Any
}

pub fn int_none() -> IntMatcher {
    IntMatcher::None
}

pub fn int_fn(matcher: impl MatcherFn<i128> + 'static) -> IntMatcher {
    IntMatcher::Fn(Box::new(matcher))
}

pub enum FloatMatcher {
    Fn(Box<dyn MatcherFn<f64>>),
    Eq(f64),
    LessThan(f64),
    GreaterThan(f64),
    Any,
    None,
}

impl FloatMatcher {
    pub fn score(&self, value: Option<&f64>) -> u16 {
        match (self, value) {
            (FloatMatcher::Eq(m), Some(v)) if v == m => 4,
            (FloatMatcher::LessThan(m), Some(v)) if v < m => 3,
            (FloatMatcher::GreaterThan(m), Some(v)) if v > m => 3,
            (FloatMatcher::None, None) => 2,
            (FloatMatcher::Any, Some(_)) => 1,
            (FloatMatcher::Fn(matcher), v) => matcher.score(v),
            _ => 0,
        }
    }
}

pub fn float_eq(num: impl Into<f64>) -> FloatMatcher {
    FloatMatcher::Eq(num.into())
}

pub fn float_lt(num: impl Into<f64>) -> FloatMatcher {
    FloatMatcher::LessThan(num.into())
}

pub fn float_gt(num: impl Into<f64>) -> FloatMatcher {
    FloatMatcher::GreaterThan(num.into())
}

pub fn float_any() -> FloatMatcher {
    FloatMatcher::Any
}

pub fn float_none() -> FloatMatcher {
    FloatMatcher::None
}

pub fn float_fn(matcher: impl MatcherFn<f64> + 'static) -> FloatMatcher {
    FloatMatcher::Fn(Box::new(matcher))
}

// Bool

pub enum BoolMatcher {
    Eq(bool),
    Any,
    None,
}

impl BoolMatcher {
    pub fn score(&self, value: Option<&bool>) -> u16 {
        match (self, value) {
            (BoolMatcher::Eq(b), Some(value)) if b.eq(value) => 3,
            (BoolMatcher::None, None) => 2,
            (BoolMatcher::Any, Some(_)) => 1,
            _ => 0,
        }
    }
}

pub fn bool_eq(value: bool) -> BoolMatcher {
    BoolMatcher::Eq(value)
}

pub fn bool_any() -> BoolMatcher {
    BoolMatcher::Any
}

pub fn bool_none() -> BoolMatcher {
    BoolMatcher::None
}

// Json - Composition of Matchers

pub enum JsonMatcher {
    Fn(Box<dyn MatcherFn<JsonValue>>),
    Null,
    Bool(BoolMatcher),
    Str(TextMatcher),
    Int(IntMatcher),
    Float(FloatMatcher),
    List(Vec<JsonMatcher>),
    Object(HashMap<String, JsonMatcher>),
}

impl JsonMatcher {
    pub fn score(&self, value: Option<&JsonValue>) -> u16 {
        match (value, self) {
            (Some(JsonValue::Null), JsonMatcher::Null) => 1,
            (Some(JsonValue::Bool(v)), JsonMatcher::Bool(matcher)) => matcher.score(Some(v)),
            (Some(JsonValue::Str(v)), JsonMatcher::Str(matcher)) => matcher.score(Some(v)),
            (Some(JsonValue::Float(v)), JsonMatcher::Float(matcher)) => matcher.score(Some(v)),
            (Some(JsonValue::Int(v)), JsonMatcher::Int(matcher)) => matcher.score(Some(v)),
            (Some(JsonValue::List(list)), JsonMatcher::List(matchers)) => {
                let mut total_score: u16 = 0;
                for (m, item) in matchers.iter().zip(list.iter()) {
                    let score = m.score(Some(item));
                    if score == 0 {
                        return 0;
                    }
                    total_score += score;
                }

                total_score
            }

            (Some(JsonValue::Object(map)), JsonMatcher::Object(matchers)) => {
                let mut total_score = 0;

                for (k, matcher) in matchers {
                    let score = matcher.score(map.get(k));
                    if score == 0 {
                        return 0;
                    }
                    total_score += score;
                }

                total_score
            }
            (_, _) => 0,
        }
    }
}

#[macro_export]
macro_rules! json_object {
    ( $( $key:expr => $value:expr ),* $(,)? ) => {{
        use std::collections::HashMap;
        use anymock::matchers::JsonMatcher;
        let mut map: HashMap<String, JsonMatcher> = HashMap::new();
        $(
            map.insert(($key).into(), ($value).into());
        )*
        JsonMatcher::Object(map)
    }};
}

#[macro_export]
macro_rules! json_list {
    ( $( $value:expr ),* $(,)? ) => {{
        let mut list: Vec<JsonMatcher> = Vec::new();
        $(
            list.push(($value).into());
        )*
        JsonMatcher::List(list)
    }};
}

pub fn json_fn(matcher: impl MatcherFn<JsonValue> + 'static) -> JsonMatcher {
    JsonMatcher::Fn(Box::new(matcher))
}

impl From<TextMatcher> for JsonMatcher {
    fn from(value: TextMatcher) -> Self {
        JsonMatcher::Str(value)
    }
}

impl From<IntMatcher> for JsonMatcher {
    fn from(value: IntMatcher) -> Self {
        JsonMatcher::Int(value)
    }
}

impl From<FloatMatcher> for JsonMatcher {
    fn from(value: FloatMatcher) -> Self {
        JsonMatcher::Float(value)
    }
}

impl From<JsonValue> for JsonMatcher {
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => JsonMatcher::Null,
            JsonValue::Bool(bool) => JsonMatcher::Bool(BoolMatcher::Eq(bool)),
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

#[cfg(test)]
mod tests {

    mod text {

        use crate::matchers::{
            text_any, text_contains, text_eq, text_fn, text_len_eq, text_len_gt, text_len_lt,
            text_none, text_not_contains, text_regex,
        };

        #[test]
        fn should_text_eq_returns_expected_scores() {
            let matcher = text_eq("Hello");

            assert!(matcher.score(Some(&String::from("Hello"))) > 0);
            assert_eq!(0, matcher.score(Some(&String::from("Hell"))));
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_text_contains_returns_expected_scores() {
            let matcher = text_contains("ell");

            assert!(matcher.score(Some(&String::from("Hello"))) > 0);
            assert_eq!(0, matcher.score(Some(&String::from("Helo"))));
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_text_not_contains_returns_expected_scores() {
            let matcher = text_not_contains("xyz");

            assert!(matcher.score(Some(&String::from("Hello"))) > 0);
            assert_eq!(0, matcher.score(Some(&String::from("Hello xyz"))));
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_text_regex_returns_expected_scores() {
            let matcher = match text_regex("^Hello$") {
                Ok(m) => m,
                Err(err) => panic!("Regex should compile: {err:?}"),
            };

            assert!(matcher.score(Some(&String::from("Hello"))) > 0);
            assert_eq!(0, matcher.score(Some(&String::from("Hell"))));
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_text_len_eq_returns_expected_scores() {
            let matcher = text_len_eq(5);

            assert!(matcher.score(Some(&String::from("Hello"))) > 0);
            assert_eq!(0, matcher.score(Some(&String::from("Hell"))));
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_text_len_gt_returns_expected_scores() {
            let matcher = text_len_gt(3);

            assert!(matcher.score(Some(&String::from("Hello"))) > 0);
            assert_eq!(0, matcher.score(Some(&String::from("Hi"))));
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_text_len_lt_returns_expected_scores() {
            let matcher = text_len_lt(5);

            assert!(matcher.score(Some(&String::from("Hell"))) > 0);
            assert_eq!(0, matcher.score(Some(&String::from("Hello"))));
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_text_any_returns_expected_scores() {
            let matcher = text_any();

            assert!(matcher.score(Some(&String::from("Hello"))) > 0);
            assert!(matcher.score(Some(&String::from(""))) > 0);
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_text_none_returns_expected_scores() {
            let matcher = text_none();

            assert!(matcher.score(None) > 0);
            assert_eq!(0, matcher.score(Some(&String::from("Hello"))));
        }

        #[test]
        fn should_text_fn_delegate_returns_expected_scores() {
            let matcher = text_fn(|value: Option<&String>| if value.is_none() { 1 } else { 0 });

            assert_eq!(1, matcher.score(None));
            assert_eq!(0, matcher.score(Some(&String::from("Hell"))));
        }

        #[test]
        fn should_preserve_text_matcher_priority_order() {
            let value = String::from("Hello");

            let eq_score = text_eq("Hello").score(Some(&value));
            let regex_score = text_regex("^Hello$").unwrap().score(Some(&value));
            let contains_score = text_contains("ell").score(Some(&value));
            let not_contains_score = text_not_contains("xyz").score(Some(&value));
            let len_eq_score = text_len_eq(5).score(Some(&value));
            let len_gt_score = text_len_gt(3).score(Some(&value));
            let len_lt_score = text_len_lt(10).score(Some(&value));
            let any_score = text_any().score(Some(&value));
            let none_score = text_none().score(None);

            assert!(
                eq_score > regex_score
                    && regex_score > contains_score
                    && contains_score > not_contains_score
                    && not_contains_score > len_eq_score
                    && len_eq_score > len_gt_score
                    && len_gt_score >= len_lt_score
                    && len_lt_score > none_score
                    && none_score > any_score
            );
        }
    }

    mod bool {
        use crate::matchers::{bool_any, bool_eq, bool_none};

        #[test]
        fn should_bool_eq_returns_expected_scores() {
            assert!(bool_eq(true).score(Some(&true)) > 0);
            assert!(bool_eq(false).score(Some(&false)) > 0);

            assert_eq!(0, bool_eq(true).score(Some(&false)));
            assert_eq!(0, bool_eq(false).score(Some(&true)));
        }

        #[test]
        fn should_bool_any_returns_expected_scores() {
            let matcher = bool_any();

            assert!(matcher.score(Some(&true)) > 0);
            assert!(matcher.score(Some(&false)) > 0);
        }

        #[test]
        fn should_bool_none_returns_expected_scores() {
            let matcher = bool_none();

            assert_eq!(0, matcher.score(Some(&true)));
            assert_eq!(0, matcher.score(Some(&false)));
        }

        #[test]
        fn should_preserve_bool_matcher_priority_order() {
            let eq_score = bool_eq(true).score(Some(&true));
            let any_score = bool_any().score(Some(&true));
            let none_score = bool_none().score(None);

            assert!(eq_score > none_score && none_score > any_score);
        }
    }

    mod int {

        use crate::matchers::{int_any, int_eq, int_fn, int_gt, int_lt, int_none};

        #[test]
        fn should_int_eq_returns_expected_scores() {
            assert!(int_eq(10).score(Some(&10)) > 0);
            assert_eq!(0, int_eq(10).score(Some(&9)));
        }

        #[test]
        fn should_int_lt_returns_expected_scores() {
            let matcher = int_lt(10);

            assert!(matcher.score(Some(&9)) > 0);
            assert_eq!(0, matcher.score(Some(&10)));
        }

        #[test]
        fn should_int_gt_returns_expected_scores() {
            let matcher = int_gt(10);

            assert!(matcher.score(Some(&100)) > 0);
            assert_eq!(0, matcher.score(Some(&10)));
        }

        #[test]
        fn should_int_any_returns_expected_scores() {
            let matcher = int_any();

            assert!(matcher.score(Some(&0)) > 0);
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_int_none_returns_expected_scores() {
            let matcher = int_none();

            assert_eq!(0, matcher.score(Some(&0)));
            assert!(matcher.score(None) > 0);
        }

        #[test]
        fn should_int_fn_returns_expected_scores() {
            let matcher = int_fn(|value: Option<&i128>| if value.is_some() { 5 } else { 0 });

            assert_eq!(5, matcher.score(Some(&42)));
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_preserve_int_matcher_priority_order() {
            let value: i128 = 10;

            let eq_score = int_eq(10).score(Some(&value));
            let lt_score = int_lt(20).score(Some(&value));
            let gt_score = int_gt(5).score(Some(&value));
            let any_score = int_any().score(Some(&value));
            let none_score = int_none().score(None);

            assert!(
                eq_score > lt_score
                    && eq_score > gt_score
                    && lt_score >= gt_score
                    && gt_score > none_score
                    && none_score > any_score
            );
        }
    }

    mod float {

        use crate::matchers::{float_any, float_eq, float_fn, float_gt, float_lt, float_none};

        #[test]
        fn should_float_eq_returns_expected_scores() {
            assert!(float_eq(10.0).score(Some(&10.0)) > 0);
            assert_eq!(0, float_eq(10.0).score(Some(&9.9)));
        }

        #[test]
        fn should_float_lt_returns_expected_scores() {
            let matcher = float_lt(10.0);

            assert!(matcher.score(Some(&9.9)) > 0);
            assert_eq!(0, matcher.score(Some(&10.1)));
        }

        #[test]
        fn should_float_gt_returns_expected_scores() {
            let matcher = float_gt(10.0);

            assert!(matcher.score(Some(&10.1)) > 0);
            assert_eq!(0, matcher.score(Some(&9.9)));
        }

        #[test]
        fn should_float_any_returns_expected_scores() {
            let matcher = float_any();

            assert!(matcher.score(Some(&0.0)) > 0);
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_float_none_returns_expected_scores() {
            let matcher = float_none();

            assert!(matcher.score(None) > 0);
            assert_eq!(0, matcher.score(Some(&10.5)));
        }

        #[test]
        fn should_float_fn_returns_expected_scores() {
            let matcher = float_fn(|value: Option<&f64>| if value.is_some() { 5 } else { 0 });

            assert_eq!(5, matcher.score(Some(&42.0)));
            assert_eq!(0, matcher.score(None));
        }

        #[test]
        fn should_preserve_float_matcher_priority_order() {
            let value: f64 = 10.0;

            let eq_score = float_eq(10.0).score(Some(&value));
            let lt_score = float_lt(20.0).score(Some(&value));
            let gt_score = float_gt(5.0).score(Some(&value));
            let any_score = float_any().score(Some(&value));
            let none_score = float_none().score(None);

            assert!(
                eq_score > lt_score
                    && eq_score > gt_score
                    && lt_score >= gt_score
                    && gt_score > none_score
                    && none_score > any_score
            );
        }
    }
}
