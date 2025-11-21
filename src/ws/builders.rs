use std::collections::HashMap;

use crate::{
    matchers::{Body, TextMatcher},
    ws::stubs::{DelayStub, ResponseStub, Stub},
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
