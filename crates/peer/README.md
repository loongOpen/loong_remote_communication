# 启动 Callee（在一个终端）

```sh
export RUST_LOG=webrtc=error,turn=error
cargo run -p peer --bin callee -- --local-id callee_1
```

# 启动 Caller（在另一个终端）

```sh
export RUST_LOG=webrtc=error,turn=error
cargo run -p peer --bin caller -- --local-id caller_1 --remote-id callee_1 --timeout-secs 3
```
