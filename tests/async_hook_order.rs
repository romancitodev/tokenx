use std::sync::{Arc, Mutex};
use tokenx::{AsyncHook, AsyncTokenProxy, AsyncTokenResolver, BoxFuture, DefaultAuditEvent};

struct EchoResolver;
impl AsyncTokenResolver for EchoResolver {
    type Token = String;
    type Resolved = String;
    type Event = DefaultAuditEvent;
    async fn resolve(&self, token: String) -> (String, DefaultAuditEvent) {
        (
            format!("resolved:{token}"),
            DefaultAuditEvent::TokenResolved {
                owner: token,
                path: "/".into(),
            },
        )
    }
}

struct Labeled(&'static str, Arc<Mutex<Vec<&'static str>>>);
impl AsyncHook<String, DefaultAuditEvent> for Labeled {
    fn pre_hook(&self, _: &String) -> BoxFuture<'_, ()> {
        let (label, log) = (self.0, self.1.clone());
        Box::pin(async move { log.lock().unwrap().push(label) })
    }
    fn post_hook(&self, _: &DefaultAuditEvent) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

struct Spy(Arc<Mutex<Vec<&'static str>>>);
impl AsyncHook<String, DefaultAuditEvent> for Spy {
    fn pre_hook(&self, _: &String) -> BoxFuture<'_, ()> {
        let log = self.0.clone();
        Box::pin(async move { log.lock().unwrap().push("pre") })
    }
    fn post_hook(&self, _: &DefaultAuditEvent) -> BoxFuture<'_, ()> {
        let log = self.0.clone();
        Box::pin(async move { log.lock().unwrap().push("post") })
    }
}

#[tokio::test]
async fn pre_fires_before_post() {
    let log = Arc::new(Mutex::new(vec![]));
    AsyncTokenProxy::new(EchoResolver)
        .add_hook(Spy(log.clone()))
        .handle_request("tok".into())
        .await;
    assert_eq!(*log.lock().unwrap(), vec!["pre", "post"]);
}

#[tokio::test]
async fn multiple_hooks_fire_in_registration_order() {
    let log = Arc::new(Mutex::new(vec![]));

    AsyncTokenProxy::new(EchoResolver)
        .add_hook(Labeled("first", log.clone()))
        .add_hook(Labeled("second", log.clone()))
        .add_hook(Labeled("third", log.clone()))
        .handle_request("tok".into())
        .await;

    assert_eq!(*log.lock().unwrap(), vec!["first", "second", "third"]);
}
