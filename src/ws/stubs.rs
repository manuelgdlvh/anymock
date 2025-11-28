use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::HashMap,
    iter::from_fn,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use rand::Rng;
use serde_json::Value;
use tungstenite::{Bytes, Message, Utf8Bytes};

use crate::matchers::{Body, BodyMatcher, TextMatcher};

#[derive(Default, Clone)]
pub struct StubsHandle {
    on_connect: Arc<RwLock<Vec<Stub>>>,
    on_message: Arc<RwLock<Vec<Stub>>>,
    on_periodical: Arc<RwLock<Vec<Stub>>>,
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
            Stub::Periodical { .. } => {
                if let Ok(mut on_periodical) = self.on_periodical.write() {
                    on_periodical.push(stub);
                }
            }
        }
    }

    pub(crate) fn on_connect(&self, headers: &HashMap<String, String>) -> Option<Msg> {
        Self::get_message(&self.on_connect, headers, None)
    }

    pub(crate) fn on_periodical(&self, headers: &HashMap<String, String>) -> Option<Vec<Msg>> {
        let messages: Vec<Msg> =
            from_fn(|| Self::get_message(&self.on_periodical, headers, None)).collect();

        (!messages.is_empty()).then_some(messages)
    }

    pub(crate) fn on_message(
        &self,
        headers: &HashMap<String, String>,
        payload: Body,
    ) -> Option<Msg> {
        Self::get_message(&self.on_message, headers, Some(&payload))
    }

    fn get_message(
        stubs: &RwLock<Vec<Stub>>,
        headers: &HashMap<String, String>,
        payload: Option<&Body>,
    ) -> Option<Msg> {
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

thread_local! {
    static PERIODICALLY_STUBS_INVOCATION_COUNT: RefCell<HashMap<String, usize>> =
        RefCell::new(HashMap::new());
}

pub enum Stub {
    Connect {
        headers: Option<HashMap<String, TextMatcher>>,
        response: Body,
    },
    Message {
        request: RequestMatcher,
        delay: Delay,
        response: Body,
    },
    Periodical {
        id: String,
        headers: Option<HashMap<String, TextMatcher>>,
        delay: Delay,
        responses: Vec<Body>,
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
            Self::Periodical {
                id,
                headers,
                responses,
                ..
            } => {
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

                let is_message_available =
                    PERIODICALLY_STUBS_INVOCATION_COUNT.with(|invocations| {
                        let map = invocations.borrow();
                        map.get(id.as_str())
                            .is_none_or(|&invocation| invocation < responses.len())
                    });
                if !is_message_available {
                    return 0;
                }

                score
            }
        }
    }

    pub fn message(&self) -> Msg {
        let available_at = match self {
            Self::Connect { .. } => Instant::now(),
            Self::Message { delay, .. } | Self::Periodical { delay, .. } => match delay {
                Delay::Fixed(delay) => Instant::now()
                    .checked_add(*delay)
                    .unwrap_or_else(Instant::now),

                Delay::Interval(from, to) => {
                    let from_as_millis: u64 = from.as_millis().try_into().unwrap_or_default();
                    let to_as_millis: u64 = to.as_millis().try_into().unwrap_or_default();
                    Instant::now()
                        .checked_add(Duration::from_millis(
                            rand::rng().random_range(from_as_millis..to_as_millis),
                        ))
                        .unwrap_or_else(Instant::now)
                }
            },
        };
        let response = match self {
            Self::Connect { response, .. } | Self::Message { response, .. } => response,
            Self::Periodical { id, responses, .. } => {
                let message_idx = PERIODICALLY_STUBS_INVOCATION_COUNT.with(|invocations| {
                    let mut map = invocations.borrow_mut();
                    let current_idx = map.entry(id.to_string()).or_insert(0);
                    let message_idx = *current_idx;
                    *current_idx += 1;
                    message_idx
                });
                responses
                    .get(message_idx)
                    .expect("Always should exist message")
            }
        };

        match response {
            Body::Json(json) => Msg(
                Message::Text(Utf8Bytes::from(&Value::from(json).to_string())),
                available_at,
            ),
            Body::PlainText(text) => {
                Msg(Message::Text(Utf8Bytes::from(text.as_str())), available_at)
            }
            Body::Binary(binary) => Msg(Message::Binary(Bytes::from(binary.clone())), available_at),
        }
    }
}

pub struct RequestMatcher {
    pub(crate) headers: Option<HashMap<String, TextMatcher>>,
    pub(crate) payload: Option<BodyMatcher>,
}

pub enum Delay {
    Fixed(Duration),
    Interval(Duration, Duration),
}

#[derive(PartialEq, Eq)]
pub struct Msg(pub(crate) Message, pub(crate) Instant);

impl PartialOrd for Msg {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Msg {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.1 >= other.1 {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}
