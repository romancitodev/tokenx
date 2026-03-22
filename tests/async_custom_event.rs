use std::sync::{Arc, Mutex};
use tokenx::{AsyncHook, AsyncTokenProxy, AsyncTokenResolver, BoxFuture};

#[derive(Debug, Clone)]
struct RequestContext {
    request_id: u64,
    service: String,
}

struct ServiceResolver;
impl AsyncTokenResolver for ServiceResolver {
    type Token = (u64, String);
    type Resolved = Result<String, &'static str>;
    type Event = RequestContext;

    async fn resolve(
        &self,
        (id, token): (u64, String),
    ) -> (Result<String, &'static str>, RequestContext) {
        let ctx = RequestContext {
            request_id: id,
            service: token.clone(),
        };
        if token.starts_with("valid-") {
            (Ok(format!("owner:{id}")), ctx)
        } else {
            (Err("unauthorized"), ctx)
        }
    }
}

struct LifecycleAudit {
    started: Arc<Mutex<Vec<u64>>>,
    finished: Arc<Mutex<Vec<u64>>>,
}

impl AsyncHook<(u64, String), RequestContext> for LifecycleAudit {
    fn pre_hook(&self, (id, _): &(u64, String)) -> BoxFuture<'_, ()> {
        let log = self.started.clone();
        let id = *id;
        Box::pin(async move {
            log.lock().unwrap().push(id);
        })
    }
    fn post_hook(&self, ctx: &RequestContext) -> BoxFuture<'_, ()> {
        let log = self.finished.clone();
        let id = ctx.request_id;
        Box::pin(async move {
            log.lock().unwrap().push(id);
        })
    }
}

struct ServiceFilter {
    target: &'static str,
    log: Arc<Mutex<Vec<u64>>>,
}

impl AsyncHook<(u64, String), RequestContext> for ServiceFilter {
    fn pre_hook(&self, (id, service): &(u64, String)) -> BoxFuture<'_, ()> {
        let log = self.log.clone();
        let target = self.target;
        let id = *id;
        let service = service.clone();
        Box::pin(async move {
            if service.contains(target) {
                log.lock().unwrap().push(id);
            }
        })
    }
    fn post_hook(&self, _: &RequestContext) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

#[tokio::test]
async fn lifecycle_audit_wraps_resolution() {
    let started = Arc::new(Mutex::new(vec![]));
    let finished = Arc::new(Mutex::new(vec![]));

    let proxy = AsyncTokenProxy::new(ServiceResolver).add_hook(LifecycleAudit {
        started: started.clone(),
        finished: finished.clone(),
    });

    proxy.handle_request((1, "valid-payments".into())).await;
    proxy.handle_request((2, "valid-auth".into())).await;
    proxy.handle_request((3, "bad-token".into())).await;

    assert_eq!(*started.lock().unwrap(), vec![1, 2, 3]);
    assert_eq!(*finished.lock().unwrap(), vec![1, 2, 3]);
}

#[tokio::test]
async fn service_filter_only_captures_matching_service() {
    let log = Arc::new(Mutex::new(vec![]));

    let proxy = AsyncTokenProxy::new(ServiceResolver).add_hook(ServiceFilter {
        target: "payments",
        log: log.clone(),
    });

    proxy.handle_request((1, "valid-payments".into())).await;
    proxy.handle_request((2, "valid-auth".into())).await;
    proxy.handle_request((3, "valid-payments-v2".into())).await;

    assert_eq!(*log.lock().unwrap(), vec![1, 3]);
}

#[tokio::test]
async fn multiple_hooks_compose_without_interference() {
    let started = Arc::new(Mutex::new(vec![]));
    let finished = Arc::new(Mutex::new(vec![]));
    let filtered = Arc::new(Mutex::new(vec![]));

    let proxy = AsyncTokenProxy::new(ServiceResolver)
        .add_hook(LifecycleAudit {
            started: started.clone(),
            finished: finished.clone(),
        })
        .add_hook(ServiceFilter {
            target: "auth",
            log: filtered.clone(),
        });

    proxy.handle_request((1, "valid-payments".into())).await;
    proxy.handle_request((2, "valid-auth".into())).await;

    assert_eq!(*started.lock().unwrap(), vec![1, 2]);
    assert_eq!(*finished.lock().unwrap(), vec![1, 2]);
    assert_eq!(*filtered.lock().unwrap(), vec![2]);
}
