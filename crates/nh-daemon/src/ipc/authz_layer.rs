//! The tower middleware that enforces authorization on every gRPC call.
//!
//! This is the single choke point where §10.3 is enforced: it reads the method
//! path from the request URI and the caller's principal from the connection's
//! peer credentials, asks the [`Authorizer`], and either forwards the request or
//! returns `PermissionDenied`. Because it wraps the whole service, no RPC can be
//! added that bypasses authorization — the guarantee is structural, not
//! per-method discipline.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use http::{Request, Response};
use tonic::body::Body;
use tower::{Layer, Service};

use crate::authz::Authorizer;
use crate::ipc::PrincipalResolver;

/// A tower [`Layer`] that installs authorization over a service.
#[derive(Clone)]
pub struct AuthzLayer {
    authorizer: Arc<Authorizer>,
    resolver: PrincipalResolver,
}

impl std::fmt::Debug for AuthzLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The resolver is a boxed closure and the authorizer is large; neither
        // is useful in a debug line.
        f.debug_struct("AuthzLayer").finish_non_exhaustive()
    }
}

impl AuthzLayer {
    /// Builds the layer over an authorizer and a principal resolver.
    #[must_use]
    pub fn new(authorizer: Arc<Authorizer>, resolver: PrincipalResolver) -> Self {
        Self {
            authorizer,
            resolver,
        }
    }
}

impl<S> Layer<S> for AuthzLayer {
    type Service = AuthzService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        AuthzService {
            inner,
            authorizer: self.authorizer.clone(),
            resolver: self.resolver.clone(),
        }
    }
}

/// The service produced by [`AuthzLayer`].
#[derive(Clone)]
pub struct AuthzService<S> {
    inner: S,
    authorizer: Arc<Authorizer>,
    resolver: PrincipalResolver,
}

impl<S> std::fmt::Debug for AuthzService<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthzService").finish_non_exhaustive()
    }
}

impl<S> Service<Request<Body>> for AuthzService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // The gRPC method path, e.g. /networkhub.v1.SystemService/GetVersion.
        let method = req.uri().path().to_owned();
        // The caller's principal, derived from the connection's peer credentials.
        let principal = (self.resolver)(&req);
        let authorizer = self.authorizer.clone();

        // Standard tower pattern: call the clone that was made ready, not the
        // field, so readiness is not lost.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            match authorizer.authorize(&method, &principal) {
                Ok(()) => inner.call(req).await,
                Err(err) => {
                    tracing::warn!(method = %method, principal = %principal.label, "denied: {err}");
                    Ok(tonic::Status::permission_denied(err.to_string()).into_http())
                }
            }
        })
    }
}
