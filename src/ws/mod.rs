use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, TcpListener},
    thread,
};

use tungstenite::accept_hdr;

use crate::{json::JsonValue, matchers::Body, ws::stubs::StubsHandle};

pub mod builders;
mod stubs;

pub struct Server {
    addr: IpAddr,
    port: u16,
    path: String,
}

impl Default for Server {
    fn default() -> Self {
        Server {
            addr: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 8080,
            path: "/".to_string(),
        }
    }
}

impl Server {
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

    pub fn start(self) -> Result<ServerHandle, std::io::Error> {
        let listener = TcpListener::bind(format!("{}:{}", self.addr, self.port))?;
        let stubs_handle = StubsHandle::default();
        let handle = ServerHandle {
            addr: self.addr,
            port: self.port,
            stubs_handle: StubsHandle::clone(&stubs_handle),
        };
        thread::spawn(|| Server::run(self, stubs_handle, listener));
        Ok(handle)
    }

    fn run(self, stubs_handle: StubsHandle, listener: TcpListener) {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(stream) => stream,
                Err(_) => {
                    continue;
                }
            };

            let mut headers: HashMap<String, String> = HashMap::new();
            let headers_ref = &mut headers;
            let callback =
                move |req: &tungstenite::handshake::server::Request,
                      response: tungstenite::handshake::server::Response| {
                    for (ref header, value) in req.headers() {
                        headers_ref.insert(header.to_string(), value.to_str().unwrap().to_string());
                    }

                    Ok(response)
                };

            let mut websocket = if let Ok(websocket) = accept_hdr(stream, callback) {
                websocket
            } else {
                continue;
            };

            thread::spawn({
                let stubs_handle = StubsHandle::clone(&stubs_handle);

                if let Some(msg) = stubs_handle.on_connect(&headers) {
                    websocket.send(msg).unwrap();
                }

                move || {
                    loop {
                        match websocket.read() {
                            Ok(msg) if msg.is_binary() => {
                                let payload = Body::Binary(msg.into_data().into());
                                if let Some(message) = stubs_handle.on_message(payload, &headers) {
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

                                if let Some(message) = stubs_handle.on_message(payload, &headers) {
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

#[derive(Clone)]
pub struct ServerHandle {
    addr: IpAddr,
    port: u16,
    stubs_handle: StubsHandle,
}

impl ServerHandle {
    pub fn register(&self, stub: stubs::Stub) {
        self.stubs_handle.register(stub);
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn addr(&self) -> String {
        self.addr.to_string()
    }
}
