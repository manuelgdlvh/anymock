use std::{collections::HashMap, marker::PhantomData, time::Duration};

use rand::distr::{Alphanumeric, SampleString};

use crate::{
    json::JsonValue,
    matchers::{Body, BodyMatcher, JsonMatcher, TextMatcher},
    ws::stubs::{Delay, RequestMatcher, Stub},
};

pub fn on_connect() -> OnConnectBuilder {
    OnConnectBuilder::default()
}

#[derive(Default)]
pub struct OnConnectBuilder {
    headers: Option<HashMap<String, TextMatcher>>,
}

impl OnConnectBuilder {
    pub fn with_header(mut self, key: impl Into<String>, matcher: TextMatcher) -> Self {
        if let Some(headers) = self.headers.as_mut() {
            headers.insert(key.into(), matcher);
        } else {
            let mut headers = HashMap::new();
            headers.insert(key.into(), matcher);
            self.headers = Some(headers);
        }

        self
    }

    pub fn returning_text(self, text: impl Into<String>) -> Stub {
        self.build(Body::PlainText(text.into()))
    }

    pub fn returning_json(self, json: impl Into<JsonValue>) -> Stub {
        self.build(Body::Json(json.into()))
    }

    pub fn returning_binary(self, buff: impl Into<Vec<u8>>) -> Stub {
        self.build(Body::Binary(buff.into()))
    }

    fn build(self, body: Body) -> Stub {
        Stub::Connect {
            headers: self.headers,
            response: body,
        }
    }
}

// Message

pub fn on_message() -> OnMessageBuilder {
    OnMessageBuilder::default()
}

#[derive(Default)]
pub struct OnMessageBuilder {
    headers: Option<HashMap<String, TextMatcher>>,
    delay: Option<Delay>,
    payload: Option<BodyMatcher>,
}

impl OnMessageBuilder {
    pub fn with_header(mut self, key: impl Into<String>, matcher: TextMatcher) -> Self {
        if let Some(headers) = self.headers.as_mut() {
            headers.insert(key.into(), matcher);
        } else {
            let mut headers = HashMap::new();
            headers.insert(key.into(), matcher);
            self.headers = Some(headers);
        }

        self
    }

    pub fn with_delay_interval_in(mut self, lower: Duration, upper: Duration) -> Self {
        match (lower, upper) {
            (lower, upper) if lower == upper => self.delay = Some(Delay::Fixed(lower)),
            (lower, upper) if lower > upper => self.delay = Some(Delay::Interval(upper, lower)),
            (lower, upper) => self.delay = Some(Delay::Interval(lower, upper)),
        }

        self
    }

    pub fn with_fixed_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(Delay::Fixed(delay));
        self
    }

    pub fn with_text_like(mut self, body: impl Into<TextMatcher>) -> Self {
        self.payload = Some(BodyMatcher::PlainText(body.into()));
        self
    }

    pub fn with_json_body_eq(mut self, body: impl Into<JsonValue>) -> Self {
        self.payload = Some(BodyMatcher::Json(JsonMatcher::from(body.into())));
        self
    }

    pub fn with_json_body_like(mut self, matcher: impl Into<JsonMatcher>) -> Self {
        self.payload = Some(BodyMatcher::Json(matcher.into()));
        self
    }

    pub fn returning_text(self, text: impl Into<String>) -> Stub {
        self.build(Body::PlainText(text.into()))
    }

    pub fn returning_json(self, json: impl Into<JsonValue>) -> Stub {
        self.build(Body::Json(json.into()))
    }

    pub fn returning_binary(self, buff: impl Into<Vec<u8>>) -> Stub {
        self.build(Body::Binary(buff.into()))
    }

    fn build(self, body: Body) -> Stub {
        Stub::Message {
            request: RequestMatcher {
                headers: self.headers,
                payload: self.payload,
            },
            delay: self
                .delay
                .unwrap_or_else(|| Delay::Fixed(Duration::from_millis(0))),
            response: body,
        }
    }
}

// Periodical Message

pub fn on_periodical() -> OnPeriodicalBuilder<NeedsBody> {
    OnPeriodicalBuilder::<NeedsBody> {
        _phantom_data: PhantomData::<NeedsBody>,
        id: None,
        headers: None,
        delay: None,
        responses: Vec::new(),
    }
}

pub struct NeedsBody;
pub struct Ready;

pub struct OnPeriodicalBuilder<T> {
    id: Option<String>,
    headers: Option<HashMap<String, TextMatcher>>,
    delay: Option<Delay>,
    responses: Vec<Body>,
    _phantom_data: PhantomData<T>,
}

impl<T> OnPeriodicalBuilder<T> {
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, matcher: TextMatcher) -> Self {
        if let Some(headers) = self.headers.as_mut() {
            headers.insert(key.into(), matcher);
        } else {
            let mut headers = HashMap::new();
            headers.insert(key.into(), matcher);
            self.headers = Some(headers);
        }

        self
    }

    pub fn with_delay_interval_in(mut self, lower: Duration, upper: Duration) -> Self {
        match (lower, upper) {
            (lower, upper) if lower == upper => self.delay = Some(Delay::Fixed(lower)),
            (lower, upper) if lower > upper => self.delay = Some(Delay::Interval(upper, lower)),
            (lower, upper) => self.delay = Some(Delay::Interval(lower, upper)),
        }

        self
    }

    pub fn with_fixed_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(Delay::Fixed(delay));
        self
    }

    pub fn returning_text(mut self, text: impl Into<String>) -> OnPeriodicalBuilder<Ready> {
        self.responses.push(Body::PlainText(text.into()));
        self.into_ready()
    }

    pub fn returning_json(mut self, json: impl Into<JsonValue>) -> OnPeriodicalBuilder<Ready> {
        self.responses.push(Body::Json(json.into()));
        self.into_ready()
    }

    pub fn returning_binary(mut self, buff: impl Into<Vec<u8>>) -> OnPeriodicalBuilder<Ready> {
        self.responses.push(Body::Binary(buff.into()));
        self.into_ready()
    }

    fn into_ready(self) -> OnPeriodicalBuilder<Ready> {
        OnPeriodicalBuilder {
            id: self.id,
            headers: self.headers,
            delay: self.delay,
            responses: self.responses,
            _phantom_data: PhantomData::<Ready>,
        }
    }
}

impl OnPeriodicalBuilder<Ready> {
    pub fn build(self) -> Stub {
        Stub::Periodical {
            id: self
                .id
                .unwrap_or_else(|| Alphanumeric.sample_string(&mut rand::rng(), 16)),
            headers: self.headers,
            delay: self
                .delay
                .unwrap_or_else(|| Delay::Fixed(Duration::from_millis(0))),
            responses: self.responses,
        }
    }
}
