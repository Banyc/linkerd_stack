use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use tower::{Layer, Service};

use crate::MakeStack;

/// `M`: a thing that makes services
#[derive(Clone, Debug)]
pub struct OnService<L, M> {
    inner: M,
    layer: L,
}
impl<L, M, Tgt> Service<Tgt> for OnService<L, M>
where
    L: Layer<M::Response> + Clone + 'static,
    M: Service<Tgt>,
    M::Future: 'static,
{
    type Response = L::Service;
    type Error = M::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, req: Tgt) -> Self::Future {
        let fut = self.inner.call(req);
        let layer = self.layer.clone();
        let next = async move {
            let svc = fut.await?;
            Ok(layer.layer(svc))
        };
        Box::pin(next)
    }
}

#[derive(Clone, Debug)]
pub struct OnServiceLayer<L>(L);
impl<L> OnServiceLayer<L> {
    pub fn new(layer: L) -> Self {
        Self(layer)
    }
}
impl<L, M> Layer<M> for OnServiceLayer<L>
where
    L: Clone,
{
    type Service = OnService<L, M>;
    fn layer(&self, inner: M) -> Self::Service {
        OnService {
            inner,
            layer: self.0.clone(),
        }
    }
}

impl<M> MakeStack<M> {
    /// The service returned from `layer` only sees the request, ignoring the target metadata.
    ///
    /// The target metadata is passed to the inner service.
    pub fn push_on_service<Tgt, Req, L>(self, layer: L) -> MakeStack<OnService<L, M>>
    where
        L: Layer<M::Response> + Clone + 'static,
        L::Service: Service<Req>,
        M: Service<Tgt>,
        M::Future: 'static,
    {
        let on_service_layer = OnServiceLayer::new(layer);
        self.push::<Tgt, Req, _>(on_service_layer)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        convert::Infallible,
        future::{ready, Future, Ready},
        pin::Pin,
        task::{Context, Poll},
    };

    use futures::pin_mut;
    use pipeline_base::Stack;
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
        tgt_mark: String,
        req_mark: String,
        resp_mark: String,
    }
    impl<S, Req> Service<Req> for TraceService<S>
    where
        Req: Trace,
        S: Service<Req, Response = Req>,
        S::Response: 'static,
        S::Future: 'static,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }
        fn call(&mut self, mut req: Req) -> Self::Future {
            req.history_mut().push(self.tgt_mark.clone());
            req.history_mut().push(self.req_mark.clone());
            let fut = self.inner.call(req);
            let tgt_mark = self.tgt_mark.clone();
            let resp_mark = self.resp_mark.clone();
            let next = async move {
                let mut resp = fut.await?;
                resp.history_mut().push(tgt_mark);
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
        type Error = Box<Infallible>;
        type Future = Ready<Result<Self::Response, Self::Error>>;
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: Req) -> Self::Future {
            ready(Ok(req))
        }
    }

    #[derive(Debug, Clone)]
    struct EchoLayer;
    impl<S> Layer<S> for EchoLayer {
        type Service = EchoService;
        fn layer(&self, _: S) -> Self::Service {
            EchoService
        }
    }

    struct MakeTrace<M> {
        inner: M,
        req_mark: String,
        resp_mark: String,
    }
    impl<M> Service<String> for MakeTrace<M>
    where
        M: Service<String>,
        M::Future: 'static,
    {
        type Response = TraceService<M::Response>;
        type Error = M::Error;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }
        fn call(&mut self, req: String) -> Self::Future {
            let fut = self.inner.call(req.clone());
            let req_mark = self.req_mark.clone();
            let resp_mark = self.resp_mark.clone();
            let next = async move {
                let svc = fut.await?;
                Ok(TraceService {
                    inner: svc,
                    tgt_mark: req,
                    req_mark,
                    resp_mark,
                })
            };
            Box::pin(next)
        }
    }

    struct MakeTraceLayer {
        req_mark: String,
        resp_mark: String,
    }
    impl<M> Layer<M> for MakeTraceLayer {
        type Service = MakeTrace<M>;
        fn layer(&self, inner: M) -> Self::Service {
            MakeTrace {
                inner,
                req_mark: self.req_mark.clone(),
                resp_mark: self.resp_mark.clone(),
            }
        }
    }

    struct VoidService;
    impl<Req> Service<Req> for VoidService {
        type Response = ();
        type Error = Infallible;
        type Future = Ready<Result<Self::Response, Self::Error>>;
        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, _: Req) -> Self::Future {
            ready(Ok(()))
        }
    }

    #[test]
    fn test_make() {
        let stack = Stack::new(VoidService);
        let make_stack = MakeStack::new::<String>(stack)
            .push_on_service::<String, TraceBody, _>(EchoLayer)
            .push::<String, TraceBody, _>(MakeTraceLayer {
                req_mark: "req_1".to_string(),
                resp_mark: "resp_1".to_string(),
            })
            .push::<String, TraceBody, _>(MakeTraceLayer {
                req_mark: "req_2".to_string(),
                resp_mark: "resp_2".to_string(),
            });
        let mut make_svc = make_stack.into_inner().into_inner();

        let target = "target".to_string();

        // Poll the make pipeline.
        let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
        let poll_ready = tower::MakeService::<String, TraceBody>::poll_ready(&mut make_svc, cx);
        let Poll::Ready(Ok(())) = poll_ready else {
            panic!("poll_ready failed");
        };

        // Call the make pipeline.
        let fut = make_svc.call(target.clone());
        pin_mut!(fut);
        let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
        let Poll::Ready(Ok(mut svc)) = fut.as_mut().poll(cx) else {
            panic!("call failed");
        };

        let req = TraceBody { history: vec![] };

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

        // Check the response.
        let expected = vec![
            target.clone(),
            "req_2".to_string(),
            target.clone(),
            "req_1".to_string(),
            target.clone(),
            "resp_1".to_string(),
            target.clone(),
            "resp_2".to_string(),
        ];
        assert_eq!(resp.history, expected);
    }
}
