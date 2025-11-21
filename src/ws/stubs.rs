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
        let mut current_score = 0;
        let mut current_stub: Option<&Stub> = None;

        if let Ok(on_connect) = self.on_connect.read() {
            for stub in on_connect.iter() {
                let score = stub.score(None, headers);
                if score > current_score {
                    current_score = score;
                    current_stub = Some(stub);
                }
            }

            current_stub.map(|stub| stub.message())
        } else {
            None
        }
    }

    pub(crate) fn on_message(
        &self,
        payload: Body,
        headers: &HashMap<String, String>,
    ) -> Option<Message> {
        let mut current_score = 0;
        let mut current_stub: Option<&Stub> = None;

        if let Ok(on_connect) = self.on_connect.read() {
            for stub in on_connect.iter() {
                let score = stub.score(Some(&payload), headers);
                if score > current_score {
                    current_score = score;
                    current_stub = Some(stub);
                }
            }

            current_stub.map(|stub| stub.message())
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
                        if let Some(header) = session_headers.get(k) {
                            let header_score = matcher.score(header);
                            if header_score != 0 {
                                score += header_score;
                            } else {
                                return 0;
                            }
                        }
                    }
                }
                score
            }
            Self::Message { request, .. } => {
                let mut score = 1;

                if let Some(header_matchers) = request.headers.as_ref() {
                    for (k, matcher) in header_matchers.iter() {
                        if let Some(header) = session_headers.get(k) {
                            let header_score = matcher.score(header);
                            if header_score != 0 {
                                score += header_score;
                            } else {
                                return 0;
                            }
                        }
                    }
                }

                let payload_score = match (payload, request.payload.as_ref()) {
                    (Some(payload), Some(matcher)) => matcher.matches(payload),
                    (Some(_), None) | (None, None) => 1,
                    (None, Some(_)) => 0,
                };

                if payload_score == 0 {
                    return 0;
                } else {
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
    headers: Option<HashMap<String, TextMatcher>>,
    payload: Option<BodyMatcher>,
}

pub struct ResponseStub {
    pub(crate) payload: Body,
    pub(crate) delay: Option<DelayStub>,
}

pub enum DelayStub {
    Fixed(u32),
    Randomized(u32, u32),
}
