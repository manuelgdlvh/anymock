// Stubs like request / response. Or PlainTextBased / PlainTextResponse.
// MockServers

// Http.
// Allow Regex/Matchers/Placeholders support. Delays.
// Request: Method, Path, Body (Plain or Json. application/*), Header, Priority (automatically handled by how much attributes are informed)
// Response: Status Code, Headers, Body (Plain or Json, application/*)
// Default fallback response.
//
//
//
// Websocket.
// Allow Regex/Matchers/Placeholders support. Delays.
// Request: Path, Body (Plain or Json. application/*), Header, Priority (automatically handled by how much attributes are informed)
// Response: Body, Headers
// Default fallback response
//
//
//
// MockServer, Stubs, Matcher for all fields.

// No need to abstract Message protocol. Each server instance can handle more than one protocol at the same time.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, TcpListener},
    sync::{
        Arc, Mutex, RwLock,
        mpsc::{Receiver, SendError, SyncSender},
    },
    thread,
    time::Duration,
};

use serde_json::{Number, Value};
use tungstenite::{Bytes, Message, Utf8Bytes, accept_hdr};

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

                    // Check On Connect

                    if let Some(msg) = stubs_registry.on_connect(&headers) {
                        websocket.send(msg).unwrap();
                    }

                    move || {
                        loop {
                            match websocket.read() {
                                Ok(msg) if msg.is_binary() => {}
                                Ok(msg) if msg.is_text() => {
                                    // Check on_message stubs
                                }
                                Ok(msg) => {}
                                Err(err) => {
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
    fallback: Arc<Mutex<Option<Stub>>>,
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
            Stub::Fallback { .. } => {
                if let Ok(mut fallback) = self.fallback.lock() {
                    *fallback = Some(stub);
                }
            }
        }
    }

    pub fn on_connect(&self, headers: &HashMap<String, String>) -> Option<Message> {
        let mut last_occurences = 0;
        let mut current_stub: Option<&Stub> = None;

        if let Ok(on_connect) = self.on_connect.read() {
            for stub in on_connect.iter() {
                if let Some(occurences) = stub.matching_rules(headers) {
                    if occurences > last_occurences {
                        last_occurences = occurences;
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
        response: Response,
    },
    Message {
        request: Request,
        response: Response,
    },
    Fallback {
        response: Response,
    },
}

impl Stub {
    pub fn matching_rules(&self, session_headers: &HashMap<String, String>) -> Option<u16> {
        match self {
            Self::Connect { headers, .. } => {
                if let Some(headers) = headers {
                    let mut matchings = 0;
                    for (k, v) in session_headers.iter() {
                        if let Some(matcher) = headers.get(k) {
                            if matcher.matches(v) {
                                matchings += 1;
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
                if let Some(headers) = request.headers.as_ref() {
                    for (k, v) in session_headers.iter() {
                        if let Some(matcher) = headers.get(k) {
                            if matcher.matches(v) {
                                matchings += 1;
                            }
                        }
                    }
                } else {
                    matchings += 1;
                }

                // Add payload also as contributor
                Some(matchings)
            }
            Self::Fallback { .. } => Some(1),
        }
    }

    pub fn message(&self) -> Message {
        match self {
            Self::Connect { response, .. }
            | Self::Message { response, .. }
            | Self::Fallback { response } => match &response.payload {
                Body::Json(json) => Message::Text(Utf8Bytes::from(&Value::from(json).to_string())),
                Body::PlainText(text) => Message::Text(Utf8Bytes::from(text.as_str())),
                Body::Binary(binary) => Message::Binary(Bytes::from(binary.clone())),
            },
        }
    }
}

pub struct Request {
    headers: Option<HashMap<String, TextMatcher>>,
    payload: Body,
}

pub struct Response {
    payload: Body,
    delay: Option<Delay>,
}

pub enum Delay {
    Fixed(u32),
    Randomized(u32, u32),
}

// No  specific of WebSocket.

enum Body {
    Json(JsonValue),
    Binary(Vec<u8>),
    PlainText(String),
}

// Use serde_json as mapping between JsonValue (outside) and Value (inside). Well known library for json parsing.
enum JsonValue {
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

// Matchers

pub enum Matcher {
    Text(TextMatcher),
    Number(NumberMatcher),
}

pub enum NumberMatcher {
    Eq(i64),
    LessThan(i64),
    GreaterThan(i64),
}

pub enum TextMatcher {
    Regex(String),
    Eq(String),
    Contains(String),
}

impl TextMatcher {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            TextMatcher::Regex(_) => false,
            TextMatcher::Eq(part) => value.eq(part),
            TextMatcher::Contains(part) => value.contains(part),
        }
    }
}
