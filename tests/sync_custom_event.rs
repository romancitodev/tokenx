use std::sync::{Arc, Mutex};
use tokenx::{Hook, TokenProxy, TokenResolver};

#[derive(Debug, Clone)]
enum ApiEvent {
    RequestStarted { id: u64, path: String },
    RequestDenied { id: u64, reason: &'static str },
}

struct ApiResolver;
impl TokenResolver for ApiResolver {
    type Token = (u64, String);
    type Resolved = bool;
    type Event = ApiEvent;

    fn resolve(&self, (id, token): (u64, String)) -> (bool, ApiEvent) {
        if token.starts_with("valid-") {
            (
                true,
                ApiEvent::RequestStarted {
                    id,
                    path: "/granted".into(),
                },
            )
        } else {
            (
                false,
                ApiEvent::RequestDenied {
                    id,
                    reason: "unauthorized",
                },
            )
        }
    }
}

struct RequestTracker {
    attempts: Arc<Mutex<Vec<u64>>>,
    denied: Arc<Mutex<Vec<u64>>>,
}

impl Hook<(u64, String), ApiEvent> for RequestTracker {
    fn pre_hook(&self, (id, _): &(u64, String)) {
        self.attempts.lock().unwrap().push(*id);
    }

    fn post_hook(&self, event: &ApiEvent) {
        if let ApiEvent::RequestDenied { id, .. } = event {
            self.denied.lock().unwrap().push(*id);
        }
    }
}

#[test]
fn tracks_attempts_and_denied_ids() {
    let attempts = Arc::new(Mutex::new(vec![]));
    let denied = Arc::new(Mutex::new(vec![]));

    let proxy = TokenProxy::new(ApiResolver).add_hook(RequestTracker {
        attempts: attempts.clone(),
        denied: denied.clone(),
    });

    let a = proxy.handle_request((1, "valid-alice".into()));
    assert!(a);
    let b = proxy.handle_request((2, "bad-token".into()));
    assert!(!b);
    let c = proxy.handle_request((3, "valid-bob".into()));
    assert!(c);

    assert_eq!(*attempts.lock().unwrap(), vec![1, 2, 3]);
    assert_eq!(*denied.lock().unwrap(), vec![2]);
}
