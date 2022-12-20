# Linkerd Stack

This repository extracts the stack component from the Linkerd proxy.

## Variations

- Prefer `tower#MakeService` over `NewService`
- Expose target types and request types in `MakeStack`
  - e.g.:
    ```rust
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
    ```
    - `String`: the target type
    - `TraceBody`: the request type

## Usage

Currently, the tests are the best way to see how to use this library.
