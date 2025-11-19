use anymock::{Body, WsMockServer, returning};
use tungstenite::connect;

#[test]
fn just_works() {
    const OUTPUT_MESSAGE: &str = "Just works!";
    const CONN_STRING: &str = "ws://localhost:8080";

    let handle = WsMockServer::default().start().unwrap();

    // Add uri and port queryable at instance
    handle
        .register(anymock::Stub::Connect {
            headers: None,
            response: returning(Body::PlainText(OUTPUT_MESSAGE.to_string())),
        })
        .expect("");

    let mut client = connect(CONN_STRING).unwrap();
    let msg = client.0.read().unwrap();
    assert!(msg.is_text());
    assert_eq!(msg.into_text().unwrap(), OUTPUT_MESSAGE);
}
