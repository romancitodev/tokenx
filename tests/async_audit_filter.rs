use std::sync::{Arc, Mutex};
use tokenx::{AsyncHook, AsyncTokenProxy, AsyncTokenResolver, BoxFuture, DefaultAuditEvent};

struct SmartResolver;
impl AsyncTokenResolver for SmartResolver {
    type Token = String;
    type Resolved = String;
    type Event = DefaultAuditEvent;

    async fn resolve(&self, token: String) -> (String, DefaultAuditEvent) {
        match token.as_str() {
            t if t.starts_with("valid-") => (
                token.clone(),
                DefaultAuditEvent::TokenResolved {
                    owner: token[6..].into(),
                    path: "/api".into(),
                },
            ),
            t if t.starts_with("denied-") => (
                String::new(),
                DefaultAuditEvent::TokenDenied {
                    token: token.clone(),
                    reason: "expired",
                },
            ),
            _ => (
                String::new(),
                DefaultAuditEvent::UpstreamError {
                    owner: String::new(),
                    status: 503,
                },
            ),
        }
    }
}

struct ErrorAuditHook(Arc<Mutex<Vec<u16>>>);
impl AsyncHook<String, DefaultAuditEvent> for ErrorAuditHook {
    fn post_hook(&self, event: &DefaultAuditEvent) -> BoxFuture<'_, ()> {
        let log = self.0.clone();
        let event = event.clone();
        Box::pin(async move {
            if let DefaultAuditEvent::UpstreamError { status, .. } = event {
                log.lock().unwrap().push(status);
            }
        })
    }
}

struct DeniedAuditHook(Arc<Mutex<Vec<String>>>);
impl AsyncHook<String, DefaultAuditEvent> for DeniedAuditHook {
    fn post_hook(&self, event: &DefaultAuditEvent) -> BoxFuture<'_, ()> {
        let log = self.0.clone();
        let event = event.clone();
        Box::pin(async move {
            if let DefaultAuditEvent::TokenDenied { token, reason } = event {
                log.lock().unwrap().push(format!("{token}: {reason}"));
            }
        })
    }
}

#[tokio::test]
async fn only_upstream_errors_are_recorded() {
    let log = Arc::new(Mutex::new(vec![]));
    let proxy = AsyncTokenProxy::new(SmartResolver).add_hook(ErrorAuditHook(log.clone()));

    proxy.handle_request("valid-alice".into()).await;
    proxy.handle_request("denied-bob".into()).await;
    proxy.handle_request("garbage".into()).await;
    proxy.handle_request("other-garbage".into()).await;

    assert_eq!(*log.lock().unwrap(), vec![503, 503]);
}

#[tokio::test]
async fn only_denied_events_are_recorded() {
    let log = Arc::new(Mutex::new(vec![]));
    let proxy = AsyncTokenProxy::new(SmartResolver).add_hook(DeniedAuditHook(log.clone()));

    proxy.handle_request("valid-carol".into()).await;
    proxy.handle_request("denied-tok".into()).await;
    proxy.handle_request("garbage".into()).await;
    proxy.handle_request("denied-other".into()).await;

    let log = log.lock().unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0], "denied-tok: expired");
    assert_eq!(log[1], "denied-other: expired");
}

#[tokio::test]
async fn hooks_composed_each_filter_independently() {
    let errors = Arc::new(Mutex::new(vec![]));
    let denied = Arc::new(Mutex::new(vec![]));

    let proxy = AsyncTokenProxy::new(SmartResolver)
        .add_hook(ErrorAuditHook(errors.clone()))
        .add_hook(DeniedAuditHook(denied.clone()));

    proxy.handle_request("valid-dave".into()).await;
    proxy.handle_request("denied-eve".into()).await;
    proxy.handle_request("bad".into()).await;

    assert_eq!(*errors.lock().unwrap(), vec![503]);
    assert_eq!(denied.lock().unwrap().len(), 1);
}
