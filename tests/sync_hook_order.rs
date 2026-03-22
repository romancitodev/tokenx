use std::sync::{Arc, Mutex};
use tokenx::{DefaultAuditEvent, Hook, TokenProxy, TokenResolver};

struct EchoResolver;
impl TokenResolver for EchoResolver {
    type Token = String;
    type Resolved = String;
    type Event = DefaultAuditEvent;
    fn resolve(&self, token: String) -> (String, DefaultAuditEvent) {
        (
            format!("resolved:{token}"),
            DefaultAuditEvent::TokenResolved {
                owner: token,
                path: "/".into(),
            },
        )
    }
}

struct Spy(Arc<Mutex<Vec<&'static str>>>);
impl Hook<String, DefaultAuditEvent> for Spy {
    fn pre_hook(&self, _: &String) {
        self.0.lock().unwrap().push("pre");
    }
    fn post_hook(&self, _: &DefaultAuditEvent) {
        self.0.lock().unwrap().push("post");
    }
}

#[test]
fn pre_fires_before_post() {
    let log = Arc::new(Mutex::new(vec![]));
    TokenProxy::new(EchoResolver)
        .add_hook(Spy(log.clone()))
        .handle_request("tok".into());
    assert_eq!(*log.lock().unwrap(), vec!["pre", "post"]);
}

#[test]
fn multiple_hooks_each_get_pre_and_post_in_order() {
    let log1 = Arc::new(Mutex::new(vec![]));
    let log2 = Arc::new(Mutex::new(vec![]));
    TokenProxy::new(EchoResolver)
        .add_hook(Spy(log1.clone()))
        .add_hook(Spy(log2.clone()))
        .handle_request("tok".into());
    assert_eq!(*log1.lock().unwrap(), vec!["pre", "post"]);
    assert_eq!(*log2.lock().unwrap(), vec!["pre", "post"]);
}

#[test]
fn no_hooks_does_not_panic() {
    TokenProxy::new(EchoResolver).handle_request("tok".into());
}

#[test]
fn hook_receives_event_produced_by_resolver() {
    let received: Arc<Mutex<Vec<DefaultAuditEvent>>> = Arc::new(Mutex::new(vec![]));

    struct Capture(Arc<Mutex<Vec<DefaultAuditEvent>>>);
    impl Hook<String, DefaultAuditEvent> for Capture {
        fn pre_hook(&self, _: &String) {}
        fn post_hook(&self, event: &DefaultAuditEvent) {
            self.0.lock().unwrap().push(event.clone());
        }
    }

    TokenProxy::new(EchoResolver)
        .add_hook(Capture(received.clone()))
        .handle_request("alice".into());

    let log = received.lock().unwrap();
    assert!(matches!(
        &log[0],
        DefaultAuditEvent::TokenResolved { owner, .. } if owner == "alice"
    ));
}
