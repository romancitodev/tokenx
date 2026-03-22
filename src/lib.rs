use std::{future::Future, pin::Pin};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Built-in audit event. Can be ignored or replaced with a custom type.
#[derive(Debug, Clone)]
pub enum DefaultAuditEvent {
    /// Token resolved successfully.
    TokenResolved { owner: String, path: String },
    /// Token not found or rejected.
    TokenDenied { token: String, reason: &'static str },
    /// Upstream responded with an error.
    UpstreamError { owner: String, status: u16 },
}

pub trait TokenResolver {
    type Token: 'static;
    type Resolved;
    type Event: Clone + Send + 'static;
    fn resolve(&self, token: Self::Token) -> (Self::Resolved, Self::Event);
}

pub trait AsyncTokenResolver: Send + Sync + 'static {
    type Token: Send + 'static;
    type Resolved: Send;
    type Event: Clone + Send + 'static;
    fn resolve(
        &self,
        token: Self::Token,
    ) -> impl Future<Output = (Self::Resolved, Self::Event)> + Send;
}

pub trait Hook<T, E>: Send + Sync + 'static {
    fn pre_hook(&self, _token: &T) {}
    fn post_hook(&self, _event: &E) {}
}

pub trait AsyncHook<T: Send, E: Send>: Send + Sync + 'static {
    fn pre_hook(&self, _token: &T) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
    fn post_hook(&self, _event: &E) -> BoxFuture<'_, ()> {
        Box::pin(async {})
    }
}

pub struct TokenProxy<T: TokenResolver> {
    resolver: T,
    hooks: Vec<Box<dyn Hook<T::Token, T::Event>>>,
}

impl<T: TokenResolver> TokenProxy<T> {
    pub fn new(resolver: T) -> Self {
        Self {
            resolver,
            hooks: Vec::new(),
        }
    }

    #[must_use]
    pub fn add_hook(mut self, hook: impl Hook<T::Token, T::Event> + 'static) -> Self {
        self.hooks.push(Box::new(hook));
        self
    }

    /// Resolves the token, then fires `pre_hook` followed by `post_hook` on all registered hooks.
    pub fn handle_request(&self, token: T::Token) -> T::Resolved {
        self.emit_pre(&token);
        let (resolved, event) = self.resolver.resolve(token);
        self.emit_post(&event);
        resolved
    }

    fn emit_pre(&self, token: &T::Token) {
        for hook in &self.hooks {
            hook.pre_hook(token);
        }
    }

    fn emit_post(&self, event: &T::Event) {
        for hook in &self.hooks {
            hook.post_hook(event);
        }
    }
}

pub struct AsyncTokenProxy<T: AsyncTokenResolver> {
    resolver: T,
    hooks: Vec<Box<dyn AsyncHook<T::Token, T::Event>>>,
}

impl<T: AsyncTokenResolver> AsyncTokenProxy<T> {
    pub fn new(resolver: T) -> Self {
        Self {
            resolver,
            hooks: Vec::new(),
        }
    }

    #[must_use]
    pub fn add_hook(mut self, hook: impl AsyncHook<T::Token, T::Event> + 'static) -> Self {
        self.hooks.push(Box::new(hook));
        self
    }

    /// Resolves the token, then fires `pre_hook` followed by `post_hook` on all registered hooks.
    pub async fn handle_request(&self, token: T::Token) -> T::Resolved {
        self.emit_pre(&token).await;
        let (resolved, event) = self.resolver.resolve(token).await;
        self.emit_post(&event).await;
        resolved
    }

    async fn emit_pre(&self, token: &T::Token) {
        for hook in &self.hooks {
            hook.pre_hook(token).await;
        }
    }

    async fn emit_post(&self, event: &T::Event) {
        for hook in &self.hooks {
            hook.post_hook(event).await;
        }
    }
}
