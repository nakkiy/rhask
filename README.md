# Rhask – Rhai-based Task Runner

Rhask is a lightweight task runner that lets you describe tasks in [Rhai](https://rhai.rs/) and execute them from a Rust CLI. Tasks and groups can be nested freely, and every entry can be invoked by its fully qualified name such as `group.task`. When a leaf name is unique, you can also call it without the prefix.

![demo](./demo.png)

---

## Quick Start (Install / Setup)

```bash
# Install from crates.io
cargo install rhask

# Example: list tasks at the project root
rhask list

# Example: run a task
rhask run <task>
```

- Place `rhaskfile.rhai` at your repository root (copy `rhaskfile_demo.rhai` or `rhaskfile_sample.rhai` to get started).
- To try the demo without touching your root, run `rhask -f ./rhaskfile_demo.rhai list`.
- You can skip the `run` subcommand and execute tasks as `rhask <task>`; it behaves exactly like `rhask run <task>`.

---

## Features

- Task definitions live in `rhaskfile.rhai` at the repository root.
- At startup Rhask searches the current directory and walks up parent directories until it finds `rhaskfile.rhai` (override with `-f` / `--file`).
- `rhask list` renders tasks and groups as a tree.
- `rhask list --flat` (or `-F`) prints each task as `full.path<TAB>Description`, which is convenient for piping into `fzf`, `peco`, etc.
- `rhask run` (or the shorthand `rhask <task>`) accepts both short names and fully qualified names. When a name is ambiguous, Rhask prints the candidates and asks you to re-run with a full path.
- Use `description()`, `actions()`, and `args()` inside `task()` or `group()` blocks to declare metadata, logic, and parameters.
- Arguments support positional values, `key=value`, `--key=value`, and `--key value` styles. Defaults and required flags are declared via `args(#{ ... })`.
- Logging is powered by `env_logger`. Regular runs stay quiet, while `RUST_LOG=debug rhask run …` surfaces the internal trace.

---

## Usage

### CLI Commands

| Command | Description |
| --- | --- |
| `rhask list [group]` | Display tasks/groups as a tree. Passing `group` limits the output to that subtree; use fully qualified names like `deploy.staging` for nested groups. |
| `rhask list --flat` / `rhask list -F` | Emit each task as `full.path` followed by an aligned description. Works with `group` filters and is ideal for piping into tools such as `fzf`. |
| `rhask run <task> [args…]` | Execute a task. Supports both short names and fully qualified names; ambiguous leaves print candidate paths and abort. You can omit `run` and type `rhask <task>` as shorthand. |
| `rhask -f <file> …` | Explicitly load a Rhai script (the flag can appear anywhere in the command). |

### Passing Arguments

Declare parameters in Rhai, e.g. `args(#{ profile: "debug", target: () })`. Positional arguments follow the declaration order (`profile` → `target`). From the CLI you can supply values using:

- Positional: `rhask run build release x86_64-unknown-linux-gnu`
- `key=value`: `rhask run build profile=release`
- `--key=value`: `rhask run build --target=x86_64-apple-darwin`
- `--key value`: `rhask run build --target wasm32-unknown-unknown`
- Any mixture of the above

Unknown keys raise an error. Parameters marked as required (`()`) must be provided or the run fails with a descriptive message.

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
| `task(name, \|\| { ... })` | Declare a task and call `description` / `actions` / `args` inside it. |
| `group(name, \|\| { ... })` | Declare a group that can contain tasks or nested sub-groups. |
| `description(text)` | Attach a description to the current task or group. |
| `actions(\|\| { ... })` | Register the execution closure. Only inside this closure may you call `trigger` / `exec`. |
| `args(#{ key1: default1, key2: (), ... })` | Declare CLI parameters. `()` signals “no default = required”. |
| `trigger(name, positional?, named?)` | Reuse another task. Provide arrays/maps for positional/named arguments. |
| `exec(command)` | Run an external command via the shell. Returns `()` on success. |

> `trigger` and `exec` are restricted to `actions()`; misusing them aborts script loading with an error.

---

## Logs & Output

- User-facing messages are separated: informational lines go to `stdout`, warnings/errors to `stderr`.
- Logging relies on `env_logger`. Set `RUST_LOG=debug rhask run …` (or `trace`, etc.) when you need troubleshooting details.
- Color output is automatically enabled when stdout is a TTY and disabled for non-TTY contexts such as CI pipelines.

---

## License

Dual-licensed under MIT OR Apache-2.0.  
See `LICENSE-MIT` and `LICENSE-APACHE` for details.
