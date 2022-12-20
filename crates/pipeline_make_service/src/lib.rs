use pipeline_base::Stack;
use tower::{Layer, MakeService};

mod on_service;

pub use on_service::{OnService, OnServiceLayer};

pub struct MakeServiceStack<S>(Stack<S>);

impl<S> MakeServiceStack<S> {
    pub fn new(stack: Stack<S>) -> Self {
        MakeServiceStack(stack)
    }

    pub fn into_inner(self) -> Stack<S> {
        self.0
    }

    /// Push an outer layer onto the stack.
    pub fn push<L>(self, layer: L) -> MakeServiceStack<L::Service>
    where
        L: Layer<S>,
    {
        let stack = self.into_inner();
        let stack = stack.push(layer);
        MakeServiceStack::new(stack)
    }

    /// Make sure the inner service is a certain `MakeService`.
    pub fn check_make<Tgt, Req>(self) -> Self
    where
        S: MakeService<Tgt, Req>,
    {
        self
    }

    /// Make sure the inner service is a certain `MakeService` and is `Clone`.
    pub fn check_make_clone<Tgt, Req>(self) -> Self
    where
        S: MakeService<Tgt, Req> + Clone,
    {
        self
    }
}
