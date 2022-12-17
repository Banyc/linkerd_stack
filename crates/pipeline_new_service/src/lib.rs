use pipeline_base::Stack;

/// Basically a `tower::MakeService`
pub trait NewService<Tgt> {
    type Service;

    fn new_service(&self, target: Tgt) -> Self::Service;
}

pub struct NewServiceStack<S>(Stack<S>);

impl<S> NewServiceStack<S> {
    pub fn new(stack: Stack<S>) -> Self {
        NewServiceStack(stack)
    }

    pub fn into_inner(self) -> Stack<S> {
        self.0
    }

    pub fn check_new<Tgt>(self) -> Self
    where
        S: NewService<Tgt>,
    {
        self
    }
}
