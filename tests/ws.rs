use std::{collections::HashMap, sync::atomic::AtomicU16};

use anymock::{
    matchers::{text_contains, text_eq},
    ws::{Server, ServerHandle, builders::on_connect},
};
use tungstenite::{Message, connect, handshake::client::Request};

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

    let msg = connect_and_read(&handle, HashMap::new());
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

    let msg = connect_and_read(
        &handle,
        map![
            "Authorization" => "AAABBBCCCDDD",
        ],
    );
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

    let msg = connect_and_read(
        &handle,
        map![
            "Authorization" => "AAABBBCCCDDD",
            "Dummy-Header" => "Dummy",
        ],
    );
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), OUTPUT_MESSAGE_3);
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

fn connect_and_read(handle: &ServerHandle, headers: HashMap<&str, &str>) -> Message {
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
    let mut client = connect(req).unwrap();
    client.0.read().unwrap()
}
