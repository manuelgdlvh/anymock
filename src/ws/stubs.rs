use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde_json::Value;
use tungstenite::{Bytes, Message, Utf8Bytes};

use crate::matchers::{Body, BodyMatcher, TextMatcher};

#[derive(Default, Clone)]
pub struct StubsHandle {
    on_connect: Arc<RwLock<Vec<Stub>>>,
    on_message: Arc<RwLock<Vec<Stub>>>,
}

impl StubsHandle {
    pub fn register(&self, stub: Stub) {
        match stub {
            Stub::Connect { .. } => {
                if let Ok(mut on_connect) = self.on_connect.write() {
                    on_connect.push(stub);
                }
            }
            Stub::Message { .. } => {
                if let Ok(mut on_message) = self.on_message.write() {
                    on_message.push(stub);
                }
            }
        }
    }

    pub(crate) fn on_connect(&self, headers: &HashMap<String, String>) -> Option<Message> {
        Self::get_message(&self.on_connect, headers, None)
    }

    pub(crate) fn on_message(
        &self,
        headers: &HashMap<String, String>,
        payload: Body,
    ) -> Option<Message> {
        Self::get_message(&self.on_message, headers, Some(&payload))
    }

    fn get_message(
        stubs: &RwLock<Vec<Stub>>,
        headers: &HashMap<String, String>,
        payload: Option<&Body>,
    ) -> Option<Message> {
        let mut current_stub: (Option<&Stub>, u16) = (None, 0);

        if let Ok(on_message) = stubs.read() {
            for stub in on_message.iter() {
                let score = stub.score(payload, headers);
                if score > current_stub.1 {
                    current_stub = (Some(stub), score);
                }
            }

            current_stub.0.map(|stub| stub.message())
        } else {
            None
        }
    }
}

// Stubs

pub enum Stub {
    Connect {
        headers: Option<HashMap<String, TextMatcher>>,
        response: ResponseStub,
    },
    Message {
        request: RequestMatcher,
        response: ResponseStub,
    },
}

impl Stub {
    pub fn score(&self, payload: Option<&Body>, session_headers: &HashMap<String, String>) -> u16 {
        match self {
            Self::Connect { headers, .. } => {
                let mut score = 1;
                if let Some(header_matchers) = headers {
                    for (k, matcher) in header_matchers.iter() {
                        let header_score = matcher.score(session_headers.get(k));
                        if header_score != 0 {
                            score += header_score;
                        } else {
                            return 0;
                        }
                    }
                }
                score
            }
            Self::Message { request, .. } => {
                let mut score = 1;

                if let Some(header_matchers) = request.headers.as_ref() {
                    for (k, matcher) in header_matchers.iter() {
                        let header_score = matcher.score(session_headers.get(k));
                        if header_score != 0 {
                            score += header_score;
                        } else {
                            return 0;
                        }
                    }
                }

                if let Some(payload_matcher) = request.payload.as_ref() {
                    let payload_score = payload_matcher.score(payload);
                    if payload_score == 0 {
                        return 0;
                    }
                    score += payload_score;
                }

                score
            }
        }
    }

    pub fn message(&self) -> Message {
        match self {
            Self::Connect { response, .. } | Self::Message { response, .. } => match &response
                .payload
            {
                Body::Json(json) => Message::Text(Utf8Bytes::from(&Value::from(json).to_string())),
                Body::PlainText(text) => Message::Text(Utf8Bytes::from(text.as_str())),
                Body::Binary(binary) => Message::Binary(Bytes::from(binary.clone())),
            },
        }
    }
}

pub struct RequestMatcher {
    pub(crate) headers: Option<HashMap<String, TextMatcher>>,
    pub(crate) payload: Option<BodyMatcher>,
}

pub struct ResponseStub {
    pub(crate) payload: Body,
}
