use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
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

struct MetricsHook {
    pre_count: Arc<AtomicU32>,
    post_count: Arc<AtomicU32>,
}

impl Hook<String, DefaultAuditEvent> for MetricsHook {
    fn pre_hook(&self, _: &String) {
        self.pre_count.fetch_add(1, Ordering::Relaxed);
    }
    fn post_hook(&self, _: &DefaultAuditEvent) {
        self.post_count.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn counts_pre_and_post_per_request() {
    let pre = Arc::new(AtomicU32::new(0));
    let post = Arc::new(AtomicU32::new(0));

    let proxy = TokenProxy::new(EchoResolver).add_hook(MetricsHook {
        pre_count: pre.clone(),
        post_count: post.clone(),
    });

    for _ in 0..3 {
        proxy.handle_request("tok".into());
    }

    assert_eq!(pre.load(Ordering::Relaxed), 3);
    assert_eq!(post.load(Ordering::Relaxed), 3);
}

#[test]
fn pre_and_post_counts_stay_in_sync() {
    let pre = Arc::new(AtomicU32::new(0));
    let post = Arc::new(AtomicU32::new(0));

    let proxy = TokenProxy::new(EchoResolver).add_hook(MetricsHook {
        pre_count: pre.clone(),
        post_count: post.clone(),
    });

    proxy.handle_request("tok".into());

    assert_eq!(
        pre.load(Ordering::Relaxed),
        post.load(Ordering::Relaxed),
        "pre and post counts must always match"
    );
}
