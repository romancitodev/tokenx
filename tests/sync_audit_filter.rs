use std::sync::{Arc, Mutex};
use tokenx::{DefaultAuditEvent, Hook, TokenProxy, TokenResolver};

struct SmartResolver;
impl TokenResolver for SmartResolver {
    type Token = String;
    type Resolved = String;
    type Event = DefaultAuditEvent;
    fn resolve(&self, token: String) -> (String, DefaultAuditEvent) {
        if let Some(token) = token.strip_prefix("valid-") {
            let owner = token.to_string();
            (
                owner.clone(),
                DefaultAuditEvent::TokenResolved {
                    owner,
                    path: "/api".into(),
                },
            )
        } else if token.starts_with("denied-") {
            (
                String::new(),
                DefaultAuditEvent::TokenDenied {
                    token,
                    reason: "expired",
                },
            )
        } else {
            (
                String::new(),
                DefaultAuditEvent::UpstreamError {
                    owner: String::new(),
                    status: 503,
                },
            )
        }
    }
}

struct DeniedAuditHook(Arc<Mutex<Vec<String>>>);
impl Hook<String, DefaultAuditEvent> for DeniedAuditHook {
    fn pre_hook(&self, _: &String) {}
    fn post_hook(&self, event: &DefaultAuditEvent) {
        if let DefaultAuditEvent::TokenDenied { token, reason } = event {
            self.0.lock().unwrap().push(format!("{token}: {reason}"));
        }
    }
}

#[test]
fn only_denied_events_are_recorded() {
    let log = Arc::new(Mutex::new(vec![]));
    let proxy = TokenProxy::new(SmartResolver).add_hook(DeniedAuditHook(log.clone()));

    proxy.handle_request("valid-alice".into());
    proxy.handle_request("denied-bad".into());
    proxy.handle_request("unknown".into());
    proxy.handle_request("denied-other".into());

    let log = log.lock().unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0], "denied-bad: expired");
    assert_eq!(log[1], "denied-other: expired");
}
