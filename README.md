<h1 align="center">AnyMock</h1>
<div align="center">
 <strong>
Mock communication protocols and data schemas for testing Rust applications.
 </strong>
</div>

<br />

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/anymock">
    <img src="https://img.shields.io/crates/v/anymock.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/anymock">
    <img src="https://img.shields.io/crates/d/anymock.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/anymock">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>
<br/>


`AnyMock` offers a unified and central way to test external protocol communications. It is inspired by the Wiremock implementations in Java and Rust.
Right now, it only supports WebSocket mocking, but support for other main communication and schema protocols is planned for the future.


<div align="center">
  <a style="display: inline" href="https://docs.rs/anymock">Documentation</a>
  <span style="display: inline"> - </span>
  <a style="display: inline" href="https://crates.io/crates/anymock">Crates.io</a>
</div>

# Table of Contents
1. [Overview](#overview)
2. [Matchers](#matchers)
3. [Responses](#responses)
4. [WebSocket Stubs](#websocket-stubs)

## Overview 

After you start the server for each implementation, you receive a Handle. This handle lets you register all the stubs.

By calling register(), you can add stubs and set up their configuration in a fluent and easy-to-read way. The matchers and stubs are typed structs, but helper functions are provided so you can configure them fluently and clearly.

The field matchers are generic for all implementations, and you can see an overview of them in the Matchers section.

The stubs depend on each protocol. For example, WebSocket uses stubs like OnConnect or OnMessage. Unlike HTTP, there is no need for fallback mechanisms.


```rust
fn main() {
    const OUTPUT_MESSAGE: &str = "Lower priority stub";
    const OUTPUT_MESSAGE_2: &str = "Middle priority stub";
    const OUTPUT_MESSAGE_3: &str = "Higher priority stub";

    let handle = Server::default().start();

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
```

## Matchers

Matchers are meant to be reused across all protocol communications. `AnyMock` not only checks if a matcher matches or not — it also provides a score for each match, giving every stub a ranking.

This lets users create different kinds of stubs: more general ones using Contains matchers, or more specific ones using Eq matchers.

Thanks to the ranking system, users do not always need to set a fixed priority. The priority can be decided automatically based on the input request.

In the future, we plan to add an option to force a fixed priority and ignore the automatic calculation.

The Fn matchers are intended to provided custom implementation of the score calculation implementing MatcherFn trait.

#### **Text**
- `Eq`
- `Contains`
- `Not Contains`
- `Regex`
- `LenEq`
- `LenGreaterThan`
- `LenLessThan`
- `Any`
- `None`
- `Fn`

#### **Int**
- `Eq`
- `GreaterThan`
- `LessThan`
- `Any`
- `None`
- `Fn`

#### **Float**
- `Eq`
- `GreaterThan`
- `LessThan`
- `Any`
- `None`
- `Fn`


#### **Binary**
- `Eq`
- `Contains`
- `Any`
- `None`
- `Fn`

#### **Json**
This matcher is special because it is built from all the matchers listed above.  
Internally, it uses the `JsonValue` type, which is similar to `serde_json::Value` representation and can be created from both `serde_json::Value` and string representations (see the tests for examples).

You can also build a `JsonMatcher` directly from a `JsonValue`. This creates a `JsonMatcher` where all fields use the `Eq` matcher by default.


To explore all matcher features, types, and helper functions in detail, check the **`matchers`** module.

## Responses

At the moment, `AnyMock` supports plain text, JSON, and binary data schema representations. In the future, more data types will be added.

All these data schemas are modeled using the `Body` type, and you can create them with the `returning_*` functions.


## WebSocket Stubs


The WebSocket stubs are **OnConnect** and **OnMessage**. You can create them using the `on_connect()` and `on_message()` helper functions located in **builders** module.


```rust

on_connect()
    .with_header("authorization", text_eq("AAABBBCCCDDD"))
    .with_header("dummy-header", text_contains("Dummy"))
    .returning_text("Just works!");

```

```rust

on_message()
    .with_json_body_like(json_object!["name" => text_eq("John"), "age" => int_gt(20), "tags" => json_list![text_contains("e")]])
    .returning_text("Just works!"),

```

By combining these stubs with the matchers described above, you can build the main use cases your application needs.

These helper functions show how configurable the WebSocket stubs are by following a simple Builder-style API.

After creating your stubs, you can register them using the **Handle** returned when the Mock Server is created.

