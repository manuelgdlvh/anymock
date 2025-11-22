use std::collections::HashMap;

use crate::{
    json::JsonValue,
    matchers::{Body, BodyMatcher, JsonMatcher, TextMatcher},
    ws::stubs::{RequestMatcher, ResponseStub, Stub},
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
            response: ResponseStub { payload: body },
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
            response: ResponseStub { payload: body },
        }
    }
}
