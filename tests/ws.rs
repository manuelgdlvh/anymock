use std::{
    collections::HashMap,
    net::TcpStream,
    sync::atomic::AtomicU16,
    time::{Duration, Instant},
};

use anymock::{
    json::JsonValue,
    json_object,
    matchers::{int_gt, text_contains, text_eq, text_len_eq},
    ws::{
        Server, ServerHandle,
        builders::{on_connect, on_message, on_periodical},
    },
};
use tungstenite::{Message, WebSocket, handshake::client::Request, stream::MaybeTlsStream};

macro_rules! map {
    ( $( $key:expr => $value:expr ),* $(,)? ) => {{
        let mut map = HashMap::new();
        $(
            map.insert($key, $value);
        )*
        map
    }};
}

static NEXT_PORT_ID: AtomicU16 = AtomicU16::new(8080);

#[test]
fn should_returns_on_connect_when_no_headers_matchers_defined() {
    const OUTPUT_MESSAGE: &str = "Just works!";

    let handle = listen();

    handle.register(on_connect().returning_text(OUTPUT_MESSAGE));

    let mut client = connect(&handle);
    let msg = client.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), OUTPUT_MESSAGE);
}

#[test]
fn should_returns_on_connect_when_headers_matchers_defined() {
    const OUTPUT_MESSAGE: &str = "Just works with headers informed!";

    let handle = listen();

    handle.register(
        on_connect()
            .with_header("authorization", text_eq("AAABBBCCCDDD"))
            .returning_text(OUTPUT_MESSAGE),
    );

    let mut client = connect_hdr(
        &handle,
        map![
            "Authorization" => "AAABBBCCCDDD",
        ],
    );

    let msg = client.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), OUTPUT_MESSAGE);
}

#[test]
fn should_returns_on_connect_message_with_highest_priority() {
    const OUTPUT_MESSAGE: &str = "Lower priority stub";
    const OUTPUT_MESSAGE_2: &str = "Middle priority stub";
    const OUTPUT_MESSAGE_3: &str = "Higher priority stub";

    let handle = listen();

    handle.register(
        on_connect()
            .with_header("authorization", text_eq("AAABBBCCCDDD"))
            .returning_text(OUTPUT_MESSAGE),
    );

    handle.register(
        on_connect()
            .with_header("authorization", text_eq("AAABBBCCCDDD"))
            .with_header("dummy-header", text_contains("mm"))
            .returning_text(OUTPUT_MESSAGE_2),
    );

    handle.register(
        on_connect()
            .with_header("authorization", text_eq("AAABBBCCCDDD"))
            .with_header("dummy-header", text_eq("Dummy"))
            .returning_text(OUTPUT_MESSAGE_3),
    );

    let mut client = connect_hdr(
        &handle,
        map![
            "Authorization" => "AAABBBCCCDDD",
            "Dummy-Header" => "Dummy",
        ],
    );

    let msg = client.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), OUTPUT_MESSAGE_3);
}

#[test]
fn should_returns_on_message_when_json_body_eq() {
    const OUTPUT_MESSAGE: &str = "Just works!";
    const JSON: &str = r#"
{
  "name": "John",
  "age": 30,
  "tags": ["dev", "rust", "json"]
}
"#;

    let handle = listen();

    handle.register(
        on_message()
            .with_json_body_eq(JsonValue::try_from(JSON).unwrap())
            .returning_text(OUTPUT_MESSAGE),
    );

    let mut client = connect(&handle);

    client.send(Message::Text(JSON.into())).unwrap();
    let msg = client.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), OUTPUT_MESSAGE);
}

#[test]
fn should_returns_on_message_when_json_body_like() {
    const OUTPUT_MESSAGE: &str = "Just works!";
    const JSON: &str = r#"
{
  "name": "John",
  "age": 30,
  "tags": ["dev", "rust", "json"]
}
"#;

    let handle = listen();

    handle.register(
        on_message()
            .with_json_body_like(json_object!["name" => text_len_eq(4), "age" => int_gt(20) ])
            .returning_text(OUTPUT_MESSAGE),
    );

    let mut client = connect(&handle);

    client.send(Message::Text(JSON.into())).unwrap();
    let msg = client.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), OUTPUT_MESSAGE);
}

#[test]
fn should_returns_on_message_when_fixed_delay_applied() {
    const LOWER_DELAY_MESSAGE: &str = "Just works with lower delay!";
    const HIGHER_DELAY_MESSAGE: &str = "Just works with higher delay!";

    let now = Instant::now();
    let lower_delay = Duration::from_secs(1);
    let higher_delay = Duration::from_secs(3);
    let handle = listen();

    handle.register(
        on_message()
            .with_text_like(text_eq(LOWER_DELAY_MESSAGE))
            .with_fixed_delay(lower_delay)
            .returning_text(LOWER_DELAY_MESSAGE),
    );

    handle.register(
        on_message()
            .with_text_like(text_eq(HIGHER_DELAY_MESSAGE))
            .with_fixed_delay(higher_delay)
            .returning_text(HIGHER_DELAY_MESSAGE),
    );

    let mut client = connect(&handle);

    client
        .send(Message::Text(LOWER_DELAY_MESSAGE.into()))
        .unwrap();
    client
        .send(Message::Text(HIGHER_DELAY_MESSAGE.into()))
        .unwrap();

    let msg = client.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), LOWER_DELAY_MESSAGE);
    assert!(now.checked_add(lower_delay).unwrap() <= Instant::now());

    let msg = client.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), HIGHER_DELAY_MESSAGE);
    assert!(now.checked_add(higher_delay).unwrap() <= Instant::now());
}

#[test]
fn should_returns_on_periodical() {
    const MESSAGE_1: &str = "Just works with first message!";
    const MESSAGE_2: &str = "Just works with second message!";

    let handle = listen();

    handle.register(
        on_periodical()
            .with_fixed_delay(Duration::from_millis(200))
            .returning_text(MESSAGE_1)
            .returning_text(MESSAGE_2)
            .build(),
    );

    let mut client_1 = connect(&handle);
    let mut client_2 = connect(&handle);

    let msg = client_1.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), MESSAGE_1);

    let msg = client_1.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), MESSAGE_2);

    let msg = client_2.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), MESSAGE_1);

    let msg = client_2.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), MESSAGE_2);
}

fn listen() -> ServerHandle {
    loop {
        if let Ok(listener) = Server::default()
            .port(NEXT_PORT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
            .start()
        {
            return listener;
        }
    }
}

fn connect(handle: &ServerHandle) -> WebSocket<MaybeTlsStream<TcpStream>> {
    connect_hdr(handle, HashMap::new())
}

fn connect_hdr(
    handle: &ServerHandle,
    headers: HashMap<&str, &str>,
) -> WebSocket<MaybeTlsStream<TcpStream>> {
    let conn_string = format!("ws://{}:{}", handle.addr(), handle.port());
    let mut req_builder = Request::builder()
        .method("GET")
        .header("Host", conn_string.as_str())
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", "Secret-Key");

    for (k, v) in headers {
        req_builder = req_builder.header(k, v);
    }

    let req = req_builder.uri(conn_string.as_str()).body(()).unwrap();
    tungstenite::connect(req).unwrap().0
}
