use pipeline_base::Stack;
use tower::MakeService;

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

    pub fn check_make<Tgt, Req>(self) -> Self
    where
        S: MakeService<Tgt, Req>,
    {
        self
    }

    pub fn check_make_clone<Tgt, Req>(self) -> Self
    where
        S: MakeService<Tgt, Req> + Clone,
    {
        self
    }
}
