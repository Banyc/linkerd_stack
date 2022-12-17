use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use tower::{Layer, Service};

use crate::MakeServiceStack;

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

impl<M> MakeServiceStack<M> {
    pub fn push_on_service<L>(self, layer: L) -> MakeServiceStack<OnService<L, M>>
    where
        L: Clone,
    {
        let on_service_layer = OnServiceLayer::new(layer);
        let stack = self.into_inner();
        let stack = stack.push(on_service_layer);
        MakeServiceStack::new(stack)
    }
}
