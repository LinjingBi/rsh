# This project is co-designed and developed by a human, ChatGPT, and Cursor.

# Cursor Task Prompt — rsh Implementation

## Task
Implement a Rust CLI tool called `rsh`, a REPL-like development shell for Rust projects.

## High-level Goal
- Allow developers to iteratively type Rust code in a terminal.
- Each input block regenerates a temporary Rust binary and runs it using Cargo.
- The interaction rhythm should feel like a Python REPL, but execution remains honest to Rust’s compilation model.

## Key Constraints (Must Follow)
1. The tool runs inside an existing Cargo project (single crate).
2. It uses the project’s existing Cargo environment:
   - `Cargo.toml`
   - `Cargo.lock`
   - `target/` build cache
3. It does **not**:
   - modify `Cargo.toml`
   - add dependencies or features
   - change toolchains
   - touch `src/main.rs`
4. It generates and overwrites:
   - `src/bin/__rsh.rs`
5. It invokes Cargo as:
   - `cargo run --bin __rsh`
   using the `cargo` executable found in `PATH`.

---

## Usage

### Building `rsh`

From this repository:

```bash
cargo build
```

This produces the binary at `target/debug/rsh`.

### Running inside a target project

From the root of a **single-crate** Cargo project where you want to experiment:

```bash
/path/to/rsh/target/debug/rsh
```

- `rsh` expects to run in a directory that contains a `Cargo.toml`.
- It will create and overwrite `src/bin/__rsh.rs` in that project.

---

## REPL Model

- Prompt:
  - `rsh> ` for the first line of a block.
  - `...> ` for continuation lines of the same block.
- Multi-line input:
  - Type as many lines as you like.
  - A **blank line** (pressing Enter on an empty line) ends the block and triggers execution.
  - Blank lines before you start a block are ignored.
- Execution per block:
  1. The block is classified into **PREAMBLE** and **BODY** lines.
  2. `src/bin/__rsh.rs` is regenerated from scratch.
  3. `cargo run --bin __rsh` is invoked.
  4. `stdout` and `stderr` from the run are printed verbatim.

### Session Model

`rsh` maintains two text buffers in memory:

1. **PREAMBLE**
   - Lines that begin with module-scope constructs:
     - `use`, `mod`, `extern crate`
     - attributes: `#![...]` and `#[...]`
     - `struct`, `enum`, `type`, `trait`, `impl`, `fn`
     - `const`, `static`
   - Classification is prefix-based only; no AST parsing.
2. **BODY**
   - All other non-empty lines.
   - Typically executable statements (`let`, function calls, `println!`, etc.).

These buffers persist for the duration of the `rsh` session and are completely regenerated into `src/bin/__rsh.rs` on each execution.

---

## Generated Code Shape

- **Sync mode (default)**:
  - Prepend all PREAMBLE lines at module scope.
  - Generate:
    ```rust
    fn __rsh_session() -> Result<(), Box<dyn std::error::Error>> {
        // BODY lines, indented
        Ok(())
    }

    fn main() {
        if let Err(e) = __rsh_session() {
            eprintln!("{}", e);
        }
    }
    ```
  - The `?` operator is allowed naturally inside `__rsh_session`.

- **Async mode**:
  - Prepend all PREAMBLE lines at module scope.
  - Generate:
    ```rust
    async fn __rsh_session() -> Result<(), Box<dyn std::error::Error>> {
        // BODY lines, indented
        Ok(())
    }
    ```
  - Wrap with runtime-specific main:
    - **Tokio**:
      ```rust
      #[tokio::main]
      async fn main() {
          if let Err(e) = __rsh_session().await {
              eprintln!("{}", e);
          }
      }
      ```
    - **async-std**:
      ```rust
      #[async_std::main]
      async fn main() {
          if let Err(e) = __rsh_session().await {
              eprintln!("{}", e);
          }
      }
      ```
    - **smol**:
      ```rust
      fn main() {
          smol::block_on(async {
              if let Err(e) = __rsh_session().await {
                  eprintln!("{}", e);
              }
          });
      }
      ```

---

## Commands

The following meta-commands are recognized **only** when typed as the first line of a new block (at `rsh> `):

- `:quit` / `:q` → exit the `rsh` session.
- `:reset` → clear PREAMBLE and BODY buffers and reset to sync mode.
- `:show` → print the current PREAMBLE, BODY, and mode.

Any other line (including lines that later fail to compile) is treated as Rust code and appended to PREAMBLE or BODY based on the prefix rules.

---

## Async Auto-Switch

- Start in **sync** mode.
- After each `cargo run --bin __rsh`:
  - If it succeeds, nothing special happens.
  - If it fails, and the stderr output looks async-related (e.g. contains `E0728`, “only allowed inside async functions”, `async fn main`, etc.), `rsh`:
    1. Scans the current project’s `Cargo.toml` (as plain text) for async runtimes:
       - Prefers `tokio`, then `async-std`, then `smol`.
    2. If a supported runtime is found:
       - Switches the session permanently to async mode with that runtime.
       - Regenerates `src/bin/__rsh.rs` in async form.
       - Reruns `cargo run --bin __rsh` once and prints its output.
    3. If no supported runtime is found:
       - Prints a clear message asking the user to add `tokio`, `async-std`, or `smol` to their `Cargo.toml`.
       - Stays in sync mode.
- Once in async mode, `rsh` does **not** attempt further automatic switches; it just prints compiler/runtime errors and returns to the prompt.

---

## Error Philosophy

- Rust compiler and runtime errors from `cargo run --bin __rsh` are printed verbatim.
- `rsh` does not attempt to fix or reinterpret user code.
- The tool itself only exits on:
  - User request (`:quit` / `:q`).
  - Non-recoverable internal errors (e.g. I/O failures, `cargo` not found).

---

## Out of Scope

- Auto-import resolution.
- Feature flag forwarding.
- Workspace support.
- Expression auto-printing.
- Persistent runtime state across sessions.
- LLM-generated code execution.

