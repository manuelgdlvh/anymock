use std::{collections::HashMap, time::Duration};

use crate::{
    json::JsonValue,
    matchers::{Body, BodyMatcher, JsonMatcher, TextMatcher},
    ws::stubs::{DelayStub, RequestMatcher, ResponseStub, Stub},
};

pub fn on_connect() -> OnConnectBuilder {
    OnConnectBuilder::default()
}

#[derive(Default)]
pub struct OnConnectBuilder {
    headers: Option<HashMap<String, TextMatcher>>,
    delay: Option<DelayStub>,
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

    pub fn with_fixed_delay(mut self, dur: Duration) -> Self {
        self.delay = Some(DelayStub::Fixed(dur));
        self
    }

    pub fn returning_text(self, text: impl Into<String>) -> Stub {
        Stub::Connect {
            headers: self.headers,
            response: ResponseStub {
                payload: Body::PlainText(text.into()),
                delay: self.delay,
            },
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
    payload: Option<BodyMatcher>,
    delay: Option<DelayStub>,
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

    pub fn with_json_body_eq(mut self, body: impl Into<JsonValue>) -> Self {
        self.payload = Some(BodyMatcher::Json(JsonMatcher::from(body.into())));
        self
    }

    pub fn with_fixed_delay(mut self, dur: Duration) -> Self {
        self.delay = Some(DelayStub::Fixed(dur));
        self
    }

    pub fn returning_text(self, text: impl Into<String>) -> Stub {
        Stub::Message {
            request: RequestMatcher {
                headers: self.headers,
                payload: self.payload,
            },
            response: ResponseStub {
                payload: Body::PlainText(text.into()),
                delay: self.delay,
            },
        }
    }
}
