## rsh Development TODO

- [x] Create Cargo crate and binary entrypoint in `src/bin/rsh.rs`.
- [x] Implement REPL loop using `rustyline` with multi-line support and commands `:q`, `:quit`, `:reset`, `:show`.
- [x] Implement preamble/body buffers and prefix-based classification of lines.
- [x] Generate `src/bin/__rsh.rs` from buffers and run `cargo run --bin __rsh`.
- [x] Detect async-related compilation errors and switch to async mode with detected runtime (tokio / async-std / smol).
- [x] Keep history only for current session and avoid modifying the target project’s `Cargo.toml` or `src/main.rs`.
- [x] Refactor project layout.
- [ ] Add a user guide.
- [ ] Add unit tests.
- [x] Bug: rollback session if async errored in second run.
