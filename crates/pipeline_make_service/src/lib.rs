use pipeline_base::Stack;
use tower::{Layer, MakeService, Service};

mod on_service;

pub use on_service::{OnService, OnServiceLayer};

/// `M`: a thing that makes services
pub struct MakeStack<M>(Stack<M>);

impl<M> MakeStack<M> {
    pub fn new<Tgt>(stack: Stack<M>) -> Self
    where
        M: Service<Tgt>,
    {
        MakeStack(stack).check()
    }

    pub fn into_inner(self) -> Stack<M> {
        self.0
    }

    /// Push an outer layer onto the stack.
    pub fn push<Tgt, Req, L>(self, layer: L) -> MakeStack<L::Service>
    where
        L: Layer<M>,
        L::Service: MakeService<Tgt, Req> + Service<Tgt>,
    {
        let stack = self.into_inner();
        let stack = stack.push(layer);
        MakeStack::new::<Tgt>(stack).check_make()
    }

    /// Make sure the inner service is a certain `Service`.
    pub fn check<Tgt>(self) -> Self
    where
        M: Service<Tgt>,
    {
        self
    }

    /// Make sure the inner service is a certain `MakeService`.
    pub fn check_make<Tgt, Req>(self) -> Self
    where
        M: MakeService<Tgt, Req>,
    {
        self
    }

    /// Make sure the inner service is a certain `MakeService` and is `Clone`.
    pub fn check_make_clone<Tgt, Req>(self) -> Self
    where
        M: MakeService<Tgt, Req> + Clone,
    {
        self
    }
}
