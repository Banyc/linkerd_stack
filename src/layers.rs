use tower::{layer::util::Identity, Layer};

/// The same as `tower#ServiceBuilder` but with a upside-down execution order.
///
/// The execution order is in line with `Stack`.
///
/// The execution order is from the bottom to the top.
pub struct Layers<L>(L);

impl Layers<Identity> {
    pub fn new() -> Self {
        Layers(Identity::new())
    }
}

impl<L> Layers<L> {
    /// Push an outer layer onto the layer stack.
    pub fn push<O>(self, outer: O) -> Layers<tower::layer::util::Stack<L, O>> {
        Layers(tower::layer::util::Stack::new(self.0, outer))
    }
}

impl<S, L: Layer<S>> Layer<S> for Layers<L> {
    type Service = L::Service;

    fn layer(&self, inner: S) -> Self::Service {
        self.0.layer(inner)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        error::Error,
        fmt::{self, Display, Formatter},
        future::{ready, Ready},
        pin::Pin,
        task::{Context, Poll},
    };

    use futures::{pin_mut, Future};
    use tower::Service;

    use super::*;

    trait Trace {
        fn history_mut(&mut self) -> &mut Vec<String>;
    }
    struct TraceBody {
        history: Vec<String>,
    }
    impl Trace for TraceBody {
        fn history_mut(&mut self) -> &mut Vec<String> {
            &mut self.history
        }
    }

    struct TraceService<S> {
        inner: S,
        req_mark: String,
        resp_mark: String,
    }
    impl<S, Req> Service<Req> for TraceService<S>
    where
        Req: Trace,
        S: Service<Req, Response = Req>,
        S::Error: Into<Box<dyn Error + Send + Sync>> + 'static,
        S::Response: 'static,
        S::Future: 'static,
    {
        type Response = S::Response;
        type Error = Box<dyn Error + Send + Sync>;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx).map_err(Into::into)
        }
        fn call(&mut self, mut req: Req) -> Self::Future {
            req.history_mut().push(self.req_mark.clone());
            let fut = self.inner.call(req);
            let resp_mark = self.resp_mark.clone();
            let next = async move {
                let mut resp = fut.await.map_err(Into::into)?;
                resp.history_mut().push(resp_mark);
                Ok(resp)
            };
            let next = Box::pin(next);
            next
        }
    }

    struct EchoService;
    impl<Req> Service<Req> for EchoService {
        type Response = Req;
        type Error = Box<EchoError>;
        type Future = Ready<Result<Self::Response, Self::Error>>;
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: Req) -> Self::Future {
            ready(Ok(req))
        }
    }
    #[derive(Debug, PartialEq, Eq)]
    struct EchoError;
    impl Display for EchoError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "echo error")
        }
    }
    impl Error for EchoError {}

    #[test]
    fn test_layers_order() {
        struct TraceLayer1;
        impl<S> Layer<S> for TraceLayer1 {
            type Service = TraceService<S>;
            fn layer(&self, inner: S) -> Self::Service {
                let req_mark = "req_mark_1".to_string();
                let resp_mark = "resp_mark_1".to_string();
                TraceService {
                    inner,
                    req_mark,
                    resp_mark,
                }
            }
        }

        struct TraceLayer2;
        impl<S> Layer<S> for TraceLayer2 {
            type Service = TraceService<S>;
            fn layer(&self, inner: S) -> Self::Service {
                let req_mark = "req_mark_2".to_string();
                let resp_mark = "resp_mark_2".to_string();
                TraceService {
                    inner,
                    req_mark,
                    resp_mark,
                }
            }
        }

        // Build the service.
        let layers = Layers::new().push(TraceLayer2).push(TraceLayer1);
        let mut svc = layers.layer(EchoService);

        // Build the request.
        let req = TraceBody {
            history: Vec::new(),
        };

        // Poll the service.
        let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
        let poll_ready =
            <TraceService<TraceService<EchoService>> as Service<TraceBody>>::poll_ready(
                &mut svc, cx,
            );
        let Poll::Ready(Ok(())) = poll_ready else {
            panic!("poll_ready failed");
        };

        // Call the service.
        let fut = svc.call(req);
        pin_mut!(fut);
        let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
        let Poll::Ready(Ok(resp)) = fut.as_mut().poll(cx) else {
            panic!("call failed");
        };
        assert_eq!(
            resp.history,
            vec!["req_mark_1", "req_mark_2", "resp_mark_2", "resp_mark_1"]
        );
    }
}
