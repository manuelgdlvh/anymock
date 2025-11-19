use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, TcpListener},
    sync::{
        Arc, RwLock,
        mpsc::{Receiver, SendError, SyncSender},
    },
    thread,
    time::Duration,
};

use serde_json::Value;
use tungstenite::{Bytes, Message, Utf8Bytes, accept_hdr};

use crate::{
    json::JsonValue,
    matchers::{BodyMatcher, TextMatcher},
};

mod json;
mod matchers;

pub struct WsMockServer {
    addr: IpAddr,
    // Add port generation. This field Optional and decouple Builder from Server
    port: u16,
    path: String,
}

impl Default for WsMockServer {
    fn default() -> Self {
        WsMockServer {
            addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 8080,
            path: "/".to_string(),
        }
    }
}

impl WsMockServer {
    pub fn addr(mut self, value: impl Into<IpAddr>) -> Self {
        self.addr = value.into();
        self
    }

    pub fn port(mut self, value: u16) -> Self {
        self.port = value;
        self
    }

    pub fn path(mut self, value: String) -> Self {
        self.path = value;
        self
    }

    pub fn start(self) -> Result<WsMockInstance, ()> {
        let listener =
            TcpListener::bind(format!("{}:{}", self.addr.to_string(), self.port)).unwrap();
        listener.set_nonblocking(true).unwrap();

        let (cmd_tx, cmd_rx) = std::sync::mpsc::sync_channel(0);
        thread::spawn(|| WsMockServer::run(self, listener, cmd_rx));
        Ok(WsMockInstance { cmd_tx })
    }

    fn run(self, listener: TcpListener, cmd_rx: Receiver<Command>) {
        // Track each client with their headers connection. This allow us to apply stubbing at connect and message level and also to periodically send message to specific ones. THINK ABOUT THIS CAPABILITIES. (Passive client)

        let stubs_registry: StubsRegistry = StubsRegistry::default();
        loop {
            match cmd_rx.recv_timeout(Duration::from_secs(1)) {
                Err(err) => match err {
                    std::sync::mpsc::RecvTimeoutError::Timeout => {}
                    std::sync::mpsc::RecvTimeoutError::Disconnected => {
                        // Disconnect on drop flag
                        break;
                    }
                },
                Ok(cmd) => match cmd {
                    Command::Stop => {
                        break;
                    }
                    Command::RegisterStub(stub) => {
                        stubs_registry.register(stub);
                    }
                },
            }

            for stream in listener.incoming() {
                let stream = if let Ok(stream) = stream {
                    stream
                } else {
                    continue;
                };

                let mut headers: HashMap<String, String> = HashMap::new();
                let headers_ref = &mut headers;
                let callback =
                    move |req: &tungstenite::handshake::server::Request,
                          response: tungstenite::handshake::server::Response| {
                        for (ref header, value) in req.headers() {
                            headers_ref
                                .insert(header.to_string(), value.to_str().unwrap().to_string());
                        }

                        Ok(response)
                    };

                let mut websocket = if let Ok(websocket) = accept_hdr(stream, callback) {
                    websocket
                } else {
                    continue;
                };

                thread::spawn({
                    let stubs_registry = StubsRegistry::clone(&stubs_registry);

                    if let Some(msg) = stubs_registry.on_connect(&headers) {
                        websocket.send(msg).unwrap();
                    }

                    move || {
                        loop {
                            match websocket.read() {
                                Ok(msg) if msg.is_binary() => {
                                    let payload = Body::Binary(msg.into_data().into());
                                    if let Some(message) =
                                        stubs_registry.on_message(payload, &headers)
                                    {
                                        websocket.send(message).unwrap();
                                    }
                                }
                                Ok(msg) if msg.is_text() => {
                                    let msg_buf = msg.into_text().unwrap();
                                    let msg_str = msg_buf.as_str();

                                    let payload = match JsonValue::try_from(msg_str) {
                                        Ok(json) => Body::Json(json),
                                        Err(_) => Body::PlainText(msg_buf.as_str().to_string()),
                                    };

                                    if let Some(message) =
                                        stubs_registry.on_message(payload, &headers)
                                    {
                                        websocket.send(message).unwrap();
                                    }
                                }
                                Ok(_) => {}
                                Err(_) => {
                                    break;
                                }
                            }
                        }
                    }
                });
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct StubsRegistry {
    on_connect: Arc<RwLock<Vec<Stub>>>,
    on_message: Arc<RwLock<Vec<Stub>>>,
}

impl StubsRegistry {
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

    pub fn on_connect(&self, headers: &HashMap<String, String>) -> Option<Message> {
        let mut current_matchings = 0;
        let mut current_stub: Option<&Stub> = None;

        if let Ok(on_connect) = self.on_connect.read() {
            for stub in on_connect.iter() {
                if let Some(matchings) = stub.matching_rules(None, headers) {
                    if matchings > current_matchings {
                        current_matchings = matchings;
                        current_stub = Some(stub);
                    }
                }
            }

            if let Some(stub) = current_stub {
                Some(stub.message())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn on_message(&self, payload: Body, headers: &HashMap<String, String>) -> Option<Message> {
        let mut current_matchings = 0;
        let mut current_stub: Option<&Stub> = None;

        if let Ok(on_connect) = self.on_connect.read() {
            for stub in on_connect.iter() {
                if let Some(matchings) = stub.matching_rules(Some(&payload), headers) {
                    if matchings > current_matchings {
                        current_matchings = matchings;
                        current_stub = Some(stub);
                    }
                }
            }

            if let Some(stub) = current_stub {
                Some(stub.message())
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub enum Command {
    Stop,
    RegisterStub(Stub),
}

#[derive(Clone)]
pub struct WsMockInstance {
    cmd_tx: SyncSender<Command>,
}

impl WsMockInstance {
    pub fn register(&self, stub: Stub) -> Result<(), SendError<Stub>> {
        self.cmd_tx
            .send(Command::RegisterStub(stub))
            .map_err(|err| match err.0 {
                Command::RegisterStub(stub) => SendError(stub),
                _ => unreachable!(),
            })
    }

    pub fn stop(self) {
        let _ = self.cmd_tx.send(Command::Stop);
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
    pub fn matching_rules(
        &self,
        payload: Option<&Body>,
        session_headers: &HashMap<String, String>,
    ) -> Option<u16> {
        match self {
            Self::Connect { headers, .. } => {
                if let Some(header_matchers) = headers {
                    let mut matchings = 0;

                    for (k, matcher) in header_matchers.iter() {
                        if let Some(header) = session_headers.get(k) {
                            if matcher.matches(header) {
                                matchings += 1;
                            } else {
                                return None;
                            }
                        }
                    }
                    Some(matchings)
                } else {
                    Some(1)
                }
            }
            Self::Message { request, .. } => {
                let mut matchings = 0;

                if let Some(header_matchers) = request.headers.as_ref() {
                    for (k, matcher) in header_matchers.iter() {
                        if let Some(header) = session_headers.get(k) {
                            if matcher.matches(header) {
                                matchings += 1;
                            } else {
                                return None;
                            }
                        }
                    }
                } else {
                    matchings += 1;
                }

                let payload_matchings = match (payload, request.payload.as_ref()) {
                    (Some(payload), Some(matcher)) => matcher.matches(payload),
                    (Some(_), None) => Some(1),
                    (None, Some(_)) => None,
                    (None, None) => Some(1),
                };

                if let Some(payload_matchings) = payload_matchings {
                    matchings += payload_matchings;
                } else {
                    return None;
                }

                Some(matchings)
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

// TODO: Generic?
pub struct ResponseStub {
    payload: Body,
    delay: Option<DelayStub>,
}

pub fn returning(payload: Body) -> ResponseStub {
    ResponseStub {
        payload,
        delay: None,
    }
}
impl ResponseStub {
    pub fn with_delay(mut self, delay: DelayStub) -> Self {
        self.delay = Some(delay);
        self
    }
}

pub enum DelayStub {
    Fixed(u32),
    Randomized(u32, u32),
}

//TODO: No  specific of WebSocket.

pub enum Body {
    Json(JsonValue),
    Binary(Vec<u8>),
    PlainText(String),
}
