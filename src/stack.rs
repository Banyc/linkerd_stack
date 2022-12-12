use tower::Layer;

#[derive(Clone, Debug)]
pub struct Stack<S>(S);
impl<S> Stack<S> {
    pub fn new(inner: S) -> Self {
        Self(inner)
    }

    /// Get the inner service.
    pub fn into_inner(self) -> S {
        self.0
    }

    /// Push an outer layer onto the stack.
    pub fn push<L>(self, layer: L) -> Stack<L::Service>
    where
        L: Layer<S>,
    {
        let service = layer.layer(self.0);
        Stack(service)
    }

    /// To restrict the type of the inner service, we can add a bound to the type parameter `S`.
    pub fn add_bound_clone(self) -> Stack<S>
    where
        S: Clone,
    {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::{ready, Ready},
        task::{Context, Poll},
    };

    use futures::{pin_mut, Future};
    use tower::Service;

    use super::*;

    #[test]
    fn test_stack_echo() {
        #[derive(Clone, Debug)]
        struct EchoService;
        impl<Req> Service<Req> for EchoService {
            type Response = Req;
            type Error = ();
            type Future = Ready<Result<Self::Response, Self::Error>>;
            fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, req: Req) -> Self::Future {
                ready(Ok(req))
            }
        }

        struct EmptyLayer;
        impl<S> Layer<S> for EmptyLayer {
            type Service = S;
            fn layer(&self, inner: S) -> Self::Service {
                inner
            }
        }

        // Build a stack of layers.
        let stack = Stack::new(EchoService {})
            .push(EmptyLayer {})
            .add_bound_clone();
        let mut service: EchoService = stack.into_inner();

        // Use the service.
        {
            let req = "hello";

            // Poll the service.
            let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
            assert_eq!(
                <EchoService as Service<&str>>::poll_ready(&mut service, cx),
                Poll::Ready(Ok(()))
            );

            // Call the service.
            let fut = service.call(req);
            pin_mut!(fut);
            let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
            let resp = fut.as_mut().poll(cx);
            assert_eq!(resp, Poll::Ready(Ok(req)));
        }
    }

    #[test]
    fn test_stack_switch() {
        #[derive(Clone, Debug)]
        struct PrintService;
        impl Service<String> for PrintService {
            type Response = ();
            type Error = ();
            type Future = Ready<Result<Self::Response, Self::Error>>;
            fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, req: String) -> Self::Future {
                println!("{}", req);
                ready(Ok(()))
            }
        }

        enum Request {
            Print(String),
            Discard,
        }

        #[derive(Clone, Debug)]
        struct SwitchService {
            print: PrintService,
        }
        impl Service<Request> for SwitchService {
            type Response = ();
            type Error = ();
            type Future = Ready<Result<Self::Response, Self::Error>>;
            fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
                Poll::Ready(Ok(()))
            }
            fn call(&mut self, req: Request) -> Self::Future {
                match req {
                    Request::Print(req) => self.print.call(req),
                    Request::Discard => ready(Ok(())),
                }
            }
        }

        // Build a stack.
        let mut service = Stack::new(SwitchService {
            print: Stack::new(PrintService {}).into_inner(),
        })
        .into_inner();

        // Use the service.
        {
            let req = Request::Print("hello".to_string());

            // Poll the service.
            let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
            assert_eq!(
                <SwitchService as Service<Request>>::poll_ready(&mut service.clone(), cx),
                Poll::Ready(Ok(()))
            );

            // Call the service.
            let fut = service.call(req);
            pin_mut!(fut);
            let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
            let resp = fut.as_mut().poll(cx);
            assert_eq!(resp, Poll::Ready(Ok(())));
        }

        // Use the service.
        {
            let req = Request::Discard;

            // Poll the service.
            let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
            assert_eq!(
                <SwitchService as Service<Request>>::poll_ready(&mut service.clone(), cx),
                Poll::Ready(Ok(()))
            );

            // Call the service.
            let fut = service.call(req);
            pin_mut!(fut);
            let cx = &mut Context::from_waker(futures::task::noop_waker_ref());
            let resp = fut.as_mut().poll(cx);
            assert_eq!(resp, Poll::Ready(Ok(())));
        }
    }
}
