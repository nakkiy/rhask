# Rhask – Rhai-based Task Runner

Rhask is a lightweight task runner that lets you describe tasks in [Rhai](https://rhai.rs/) and execute them from a Rust CLI. Tasks and groups can be nested arbitrarily, and every item can be invoked via a fully qualified name such as `group.task`.

---

## Features

- Task definition lives in `rhaskfile.rhai` at your project root.
- If `--file/-f` is not provided, Rhask searches the current directory and then walks up parent directories until it finds `rhaskfile.rhai`.
- `rhask list` renders tasks and groups as a tree; warnings and errors are emitted to `stderr`, listings to `stdout`.
- `rhask run` accepts both leaf names and fully qualified names. When a name is ambiguous, Rhask prints the candidate list and asks you to re-run with a full path.
- `description()`, `actions()`, and `args()` are available inside `task()`/`group()` blocks so you can declare metadata, implement logic, and describe parameters.
- CLI arguments can be mixed (`positional`, `key=value`, `--key=value`, `--key value`) and defaults/required values can be declared via `args(#{ ... })`.
- Logging uses `env_logger`; normal runs are quiet, while `RUST_LOG=debug rhask run …` exposes internal tracing.

---

## Installation / Build

The crate is not published on crates.io yet. Build and run from sources:

```bash
# Build dependencies
cargo build

# List registered tasks
cargo run -- list

# Execute a task
cargo run -- run build
```

---

## CLI Usage

| Command | Description |
| --- | --- |
| `rhask list [group]` | Show all tasks/groups as a tree. Provide `group` to display only that subtree. |
| `rhask run <task> [args…]` | Execute a task. Supports short names and fully qualified names; ambiguous names print candidate paths. |
| `rhask -f <file> …` | Explicitly load the given Rhai script (the flag can appear anywhere). |

### Passing Arguments

Declare parameters in Rhai, e.g. `args(#{ profile: "debug", target: () })`, and invoke them from the CLI using any of the following styles:

- Positional: `rhask run build release`
- `key=value`: `rhask run build profile=release`
- `--key=value`: `rhask run build --target=x86_64-apple-darwin`
- `--key value`: `rhask run build --target wasm32-unknown-unknown`
- You can mix styles within the same command.

Unknown keys produce an error. Parameters marked as required (`()`) must be provided; otherwise the run fails with a descriptive message.

---

## Task Definition Example

```rhai
task("build", || {
    description("Build the project");
    args(#{
        profile: "debug",
        target: "x86_64-unknown-linux-gnu"
    });
    actions(|profile, target| {
        print("build => profile:" + profile + ", target:" + target);
    });
});

group("release_flow", || {
    description("Release tasks");
    task("package", || {
        actions(|| { exec("cargo package"); });
    });
});
```

### Rhai Helpers

| Helper | Description |
| --- | --- |
| `task(name, \|\| { ... })` | Declare a task. Use `description`/`actions`/`args` inside. |
| `group(name, \|\| { ... })` | Declare a group. You can nest tasks or sub-groups inside. |
| `description(text)` | Attach a description to the current task or group. |
| `actions(\|\| { ... })` | Register the execution closure. Only here can you call `trigger`/`exec`. |
| `args(#{ key: default, required: () })` | Declare CLI parameters and defaults/required flags. |
| `trigger(name, positional?, named?)` | Reuse another task. Provide arrays/maps for positional/named args. |
| `exec(command)` | Run an external command through the shell. Returns `()` on success. |

> **Note**: `trigger` and `exec` are only allowed inside `actions()`. Misuse raises an error when the script is loaded.

---

## Logs & Output

- User-facing messages are routed through the `printer` module: informational lines go to `stdout`, warnings/errors to `stderr`.
- Logging relies on `env_logger`. Set `RUST_LOG=debug` (or `trace`, etc.) to inspect the execution flow.
- Colorful output is automatically enabled when stdout is a TTY; it is suppressed for non-TTY contexts (pipes, CI logs, etc.).

---

## License

Dual-licensed under MIT OR Apache-2.0.  
See `LICENSE-MIT` and `LICENSE-APACHE` for details.
