use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
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

struct MetricsHook {
    pre_count: Arc<AtomicU32>,
    post_count: Arc<AtomicU32>,
}

impl AsyncHook<String, DefaultAuditEvent> for MetricsHook {
    fn pre_hook(&self, _: &String) -> BoxFuture<'_, ()> {
        let c = self.pre_count.clone();
        Box::pin(async move {
            c.fetch_add(1, Ordering::Relaxed);
        })
    }
    fn post_hook(&self, _: &DefaultAuditEvent) -> BoxFuture<'_, ()> {
        let c = self.post_count.clone();
        Box::pin(async move {
            c.fetch_add(1, Ordering::Relaxed);
        })
    }
}

#[tokio::test]
async fn counts_pre_and_post_per_request() {
    let pre = Arc::new(AtomicU32::new(0));
    let post = Arc::new(AtomicU32::new(0));

    let proxy = AsyncTokenProxy::new(EchoResolver).add_hook(MetricsHook {
        pre_count: pre.clone(),
        post_count: post.clone(),
    });

    for _ in 0..5 {
        proxy.handle_request("tok".into()).await;
    }

    assert_eq!(pre.load(Ordering::Relaxed), 5);
    assert_eq!(post.load(Ordering::Relaxed), 5);
}

#[tokio::test]
async fn pre_and_post_stay_in_sync() {
    let pre = Arc::new(AtomicU32::new(0));
    let post = Arc::new(AtomicU32::new(0));

    let proxy = AsyncTokenProxy::new(EchoResolver).add_hook(MetricsHook {
        pre_count: pre.clone(),
        post_count: post.clone(),
    });

    proxy.handle_request("tok".into()).await;

    assert_eq!(
        pre.load(Ordering::Relaxed),
        post.load(Ordering::Relaxed),
        "pre and post counts must always match"
    );
}
