use pipeline_base::Stack;
use tower::MakeService;

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
}
