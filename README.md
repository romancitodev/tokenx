# 🛡️ tokenx

**tokenx** is a lightweight, flexible Rust library that implements the **Token Proxy** pattern. It handles token resolution lifecycles, allowing you to transparently inject auditing, metrics, and logging logic into your authentication flow.

---

## 📑 Table of Contents

- [What is a Token Proxy?](#-what-is-a-token-proxy)
- [Library Scope](#-library-scope)
- [Web Usage Example](#-web-usage-example)
- [Features](#-features)

---

## ❓ What is a Token Proxy?

In modern web architecture, services often receive requests with opaque identifiers (like API keys or session IDs). Before your core business logic can run, you need to transform these "raw" tokens into actionable identities (like a `User` struct or an internal JWT).

The **Token Proxy** pattern acts as the middleware for this task:
1.  **Intercepts** the raw token.
2.  **Resolves** it (queries a DB, cache, or external auth service).
3.  **Returns** the resolved identity/token.

`tokenx` standardizes this flow, giving you **Hooks** to execute logic *before* and *after* resolution without cluttering your endpoints.

## 🎯 Library Scope

`tokenx` provides the **skeleton**, not the database driver.

1.  **You provide the logic**: You implement the `Resolver` (e.g., query Postgres, Redis, or validate a signature).
2.  **We provide the control**: `tokenx` manages the execution of:
    -   **Pre-hooks**: Run *before* resolution (receiving the raw token).
    -   **Post-hooks**: Run *after* resolution (receiving the audit event produced by the resolver).

## 🚀 Usage Example

**Scenario**: You have an **API Gateway (Service A)**. It receives a public API Key from a client. It needs to resolve this key into an internal JWT to make a request to a **Microservice (Service B)**.

### 1. Define the Resolver

We implement `AsyncTokenResolver`. Input: API Key. Output: Internal JWT.

```rust
use tokenx::{AsyncTokenResolver, DefaultAuditEvent};

struct ApiKeyResolver;

// Simulating a database lookup
impl AsyncTokenResolver for ApiKeyResolver {
    type Token = String;          // Input: Public API Key
    type Resolved = Option<String>; // Output: Internal JWT for Service B
    type Event = DefaultAuditEvent; // Audit event

    async fn resolve(&self, api_key: String) -> (Option<String>, DefaultAuditEvent) {
        // In a real app, you'd check Redis or Postgres here.
        if api_key == "pk_live_12345" {
            (
                Some("eyJhbGciOiJIUzI1NiJ9...".to_string()), // The internal JWT
                DefaultAuditEvent::TokenResolved { 
                    owner: "Merchant-X".into(), 
                    path: "/orders".into() 
                }
            )
        } else {
            (
                None,
                DefaultAuditEvent::TokenDenied { 
                    token: api_key, 
                    reason: "Invalid API Key" 
                }
            )
        }
    }
}
```

### 2. Add Monitoring (Hooks)

We want to count attempts using a Hook.

```rust
use tokenx::{AsyncHook, BoxFuture, DefaultAuditEvent};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

struct MetricsHook {
    attempts: Arc<AtomicU32>,
}

impl AsyncHook<String, DefaultAuditEvent> for MetricsHook {
    // Pre-hook: Runs BEFORE resolution. Good for rate limiting or metrics.
    fn pre_hook(&self, _token: &String) -> BoxFuture<'_, ()> {
        let c = self.attempts.clone();
        Box::pin(async move {
            c.fetch_add(1, Ordering::Relaxed);
        })
    }
}
```

### 3. The Gateway Service Logic

Tie it all together in your request handler. This example simulates an API Gateway resolving a public key to an internal JWT to query an upstream LLM service.

```rust
#[tokio::main]
async fn main() {
    // 1. Setup Resolver & Hooks
    let resolver = ApiKeyResolver;
    let metrics = MetricsHook { attempts: Arc::new(AtomicU32::new(0)) };

    // 2. Build the Proxy
    let token_proxy = tokenx::AsyncTokenProxy::new(resolver)
        .add_hook(metrics);

    // 3. Simulate Incoming Request (e.g., from a client SDK)
    let incoming_api_key = "pk_live_12345".to_string();
    println!("Service A: Received request with key '{}'", incoming_api_key);

    // 4. Resolve the token
    if let Some(upstream_jwt) = token_proxy.handle_request(incoming_api_key).await {
        println!("Resolution Success! Forwarding to Upstream...");
    // 5. Call Upstream Service B (e.g., an LLM Engine)
        let response = reqwest::Client::new()
            .post("https://llm-engine.internal/v1/chat/completions")
            .bearer_auth(upstream_jwt) // <--- Authentication injected here
            .json(&serde_json::json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "Explain Rust traits"}]
            }))
            .send()
            .await?;
        println!("Request sent to Service B with Authorization: Bearer {}", upstream_jwt);
    } else {
        println!("Resolution Failed: Access Denied");
    }
}
```

## ✨ Features

-   **🔌 Agnostic**: Works with any input (String, Structs) and any output (Result, Option, User).
-   **⚡ Async & Sync**: First-class support for `tokio`/`async-std` via `AsyncTokenResolver`, plus blocking support.
-   **📝 Typed Events**: The resolver emits strongly-typed events (enums/structs), ensuring your audit logs are structured and reliable.
-   **🔗 Chainable Hooks**: Add as many hooks as you need (Logging, Metrics, Tracing).
