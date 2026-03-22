use tokenx::{AsyncTokenProxy, AsyncTokenResolver, DefaultAuditEvent, TokenProxy, TokenResolver};

struct SyncGateway;
impl TokenResolver for SyncGateway {
    type Token = String;
    type Resolved = Result<String, &'static str>;
    type Event = DefaultAuditEvent;

    fn resolve(&self, token: String) -> (Self::Resolved, Self::Event) {
        if let Some(token) = token.strip_prefix("valid-") {
            let owner = token.to_string();
            (
                Ok(format!("owner:{owner}")),
                DefaultAuditEvent::TokenResolved {
                    owner,
                    path: "/".into(),
                },
            )
        } else {
            (
                Err("invalid token"),
                DefaultAuditEvent::TokenDenied {
                    token,
                    reason: "invalid token",
                },
            )
        }
    }
}

struct AsyncGateway;
impl AsyncTokenResolver for AsyncGateway {
    type Token = String;
    type Resolved = Result<String, &'static str>;
    type Event = DefaultAuditEvent;

    async fn resolve(&self, token: String) -> (Self::Resolved, Self::Event) {
        if let Some(token) = token.strip_prefix("valid-") {
            let owner = token.to_string();
            (
                Ok(format!("owner:{owner}")),
                DefaultAuditEvent::TokenResolved {
                    owner,
                    path: "/".into(),
                },
            )
        } else {
            (
                Err("invalid token"),
                DefaultAuditEvent::TokenDenied {
                    token,
                    reason: "invalid token",
                },
            )
        }
    }
}

#[test]
fn sync_valid_token_resolves_to_owner() {
    let proxy = TokenProxy::new(SyncGateway);
    assert_eq!(
        proxy.handle_request("valid-alice".into()),
        Ok("owner:alice".to_string())
    );
}

#[test]
fn sync_invalid_token_is_rejected() {
    let proxy = TokenProxy::new(SyncGateway);
    assert_eq!(proxy.handle_request("garbage".into()), Err("invalid token"));
}

#[tokio::test]
async fn async_valid_token_resolves_to_owner() {
    let proxy = AsyncTokenProxy::new(AsyncGateway);
    assert_eq!(
        proxy.handle_request("valid-bob".into()).await,
        Ok("owner:bob".to_string())
    );
}

#[tokio::test]
async fn async_invalid_token_is_rejected() {
    let proxy = AsyncTokenProxy::new(AsyncGateway);
    assert_eq!(
        proxy.handle_request("garbage".into()).await,
        Err("invalid token")
    );
}
