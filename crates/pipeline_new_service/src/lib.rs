use pipeline_base::Stack;

/// Basically a `tower::MakeService`
pub trait NewService<T> {
    type Service;

    fn new_service(&self, target: T) -> Self::Service;
}

pub struct NewServiceStack<S>(Stack<S>);

impl<S> NewServiceStack<S> {
    pub fn new(stack: Stack<S>) -> Self {
        NewServiceStack(stack)
    }

    pub fn into_inner(self) -> Stack<S> {
        self.0
    }

    pub fn check_new<T>(self) -> Self
    where
        S: NewService<T>,
    {
        self
    }
}
