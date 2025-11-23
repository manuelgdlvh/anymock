
# WebSocket Roadmap (High-Level Overview)

This roadmap outlines the major capability areas planned for WebSocket mocking in `anymock`.  
It intentionally excludes implementation details and focuses only on conceptual coverage and behavior.

---

## 1. Matchers Completeness

Goal: Provide a full and flexible set of matchers.

Planned areas:

- Add more matchers taking Wiremock Java implementation as reference
- Support for combining matchers  
  (logical AND / OR / NOT)

---

## 2. Delay & Fault Simulation

Goal: Enable realistic simulation of latency and error conditions throughout the WebSocket lifecycle.

---

## 3. Custom Function Matchers

Goal: Allow users to define their own programmatic matchers that participate in stub selection.

Planned areas:

- Custom functions that evaluate incoming messages or handshake data  

---

## 4. Custom Function Body Generators

Goal: Enable dynamic and stateful generation of outgoing WebSocket messages.


---

## 5. Periodic Message Stubs

Goal: Allow the mock to emit messages automatically at defined intervals.

Planned areas:

- Periodic broadcasting of messages to one or more active connections  
- Fixed-interval and quantity of message production

---

## 6. Administration & Verification Features

Goal: Provide visibility and control over stubs, interactions, and history.

Planned areas:

- Add optional identifiers for Stubs
- Resetting or remove mocks
- Retrieving verification data  
  (e.g., which stubs were triggered, how often, and by which connections)


---

## 7. Documentation & Examples

Goal: Deliver comprehensive, easy-to-follow documentation for all WebSocket mocking capabilities.

Planned areas:

- Conceptual guides for each major feature  
- Scenarios demonstrating realistic WebSocket testing use-cases  
- Clear examples combining matching, dynamic generation, delays, and verification

---
