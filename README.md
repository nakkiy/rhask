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
- Declare `default_task("group.task")` at the top level (or inside imported files) to automatically run that task when `rhask` is invoked without additional arguments. If `default_task` is not set, `rhask` with no arguments falls back to listing tasks.

---

## Features

- Task definitions live in `rhaskfile.rhai` at the repository root.
- At startup Rhask searches the current directory and walks up parent directories until it finds `rhaskfile.rhai` (override with `-f` / `--file`).
- `rhask list` renders tasks and groups as a tree.
- `rhask list --flat` (or `-F`) prints each task as `full.path` followed by a space-aligned description (colorized on TTY), which is convenient for piping into `fzf`, `peco`, etc.
- `rhask run` (or the shorthand `rhask <task>`) accepts both short names and fully qualified names. When a name is ambiguous, Rhask prints the candidates and asks you to re-run with a full path.
- Use `description()`, `actions()`, and `args()` inside `task()` or `group()` blocks to declare metadata, logic, and parameters.
- Arguments support positional values, `key=value`, `--key=value`, and `--key value` styles. Defaults and required flags are declared via `args(#{ ... })`.
- `default_task("group.task")` (callable from the root file or any imported file) lets you define what happens when `rhask` is executed with no explicit subcommand; it runs the configured task or falls back to `rhask list` when unset.
- `dir("path")` (callable once per task) pins the working directory for **that** task only. Relative paths are resolved from the directory that hosts `rhaskfile.rhai`, while absolute paths are honored as-is. External commands (`exec(cmd([...]).build())`) respect the task’s directory; triggered tasks run in their own `dir()` (or, if unset, the shell’s launch directory).
- `cmd(["cmd", "arg", ...])` / `.pipe()` lets you describe structured pipelines without shell-specific syntax. You can attach `.env()` or `.allow_exit_codes()` settings, stream output via `.run_stream()`, and execute them safely via `exec()` / `exec_stream()` so failures propagate like ordinary Rhai errors.
- Logging is powered by `env_logger`. Regular runs stay quiet, while `RUST_LOG=debug rhask run …` surfaces the internal trace.

---

## Usage

### CLI Commands

| Command | Description |
| --- | --- |
| `rhask list [group]` | Display tasks/groups as a tree. Passing `group` limits the output to that subtree; use fully qualified names like `deploy.staging` for nested groups. |
| `rhask list --flat` / `rhask list -F` | Emit each task as `full.path` followed by a space-aligned description (colorized on TTY). Works with `group` filters and is ideal for piping into tools such as `fzf`. |
| `rhask run <task> [args…]` | Execute a task. Supports both short names and fully qualified names; ambiguous leaves print candidate paths and abort. You can omit `run` and type `rhask <task>` as shorthand. |
| `rhask -f <file> …` | Explicitly load a Rhai script. Place `-f/--file` before the subcommand or task name (e.g. `rhask -f ./demo.rhai list`, `rhask -f ./demo.rhai run build`, `rhask -f ./demo.rhai build`). |
| `rhask` (no arguments) | Runs the task defined via `default_task("...")`. When no default task is configured it behaves like `rhask list`. |
| `rhask completions <shell>` | Generate Bash/Zsh/Fish completion scripts (see instructions below). |

### Passing Arguments

Declare parameters in Rhai, e.g. `args(#{ target: (), profile: "debug" })`. Positional arguments follow the lexicographic order of the keys (this example maps to `profile` → `target`). `()` means “no default = required”. Favor named forms when order matters, and supply values from the CLI using:

- Positional (key order): `rhask run build release x86_64-unknown-linux-gnu`
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
        target: (),
        profile: "debug"
    });
    actions(|profile, target| {
        print("build => profile:" + profile + ", target:" + target);
    });
});

group("release_flow", || {
    description("Release tasks");
    task("package", || {
        actions(|| {
            exec(cmd(["cargo", "package"]).build());
        });
    });
});
```

### Rhai Helpers

| Helper | Description |
| --- | --- |
| `task(name, \|\| { ... })` | Declare a task and call `description` / `actions` / `args` inside it. |
| `group(name, \|\| { ... })` | Declare a group that can contain tasks or nested sub-groups. |
| `description(text)` | Attach a description to the current task or group (tasks may call it only once). |
| `actions(\|\| { ... })` | Register the execution closure for a task (only valid inside `task()` and callable once per task). `trigger()` and helpers such as `exec(cmd([...]).build())` belong inside this closure. |
| `args(#{ key1: default1, key2: (), ... })` | Declare CLI parameters for the surrounding task (only valid inside `task()`). `()` signals “no default = required”. Each task may call this helper only once. |
| `dir(path)` | Only valid inside `task()`. Each task may call it at most once. Paths resolve from the directory that contains `rhaskfile.rhai` (unless they start with `/`, in which case they are treated as absolute). Missing or non-directory paths fail during script load. |
| `default_task("full.path")` | Call once at the top level (root file or imported files). When `rhask` runs without subcommands it executes this task; otherwise it falls back to listing tasks. |
| `trigger(name, positional?, named?)` | Reuse another task. Provide arrays/maps for positional/named arguments. Triggered tasks run within their own `dir()` (if any); parent settings are not inherited. |
| `cmd([cmd, arg, ...])` | Build a structured command/pipeline by chaining `.pipe()` / `.env()` etc., then execute it with `exec(...)` or `exec_stream(...)`. Both helpers return a map (`#{ success, status, stdout, stderr, duration_ms }`). |
| `exec(pipeline)` / `exec_stream(pipeline, stdout_cb?, stderr_cb?)` | Only valid inside `actions()`. Execute a pipeline constructed via `cmd(...).pipe(...).build()`. `exec` mirrors the command’s output to the console and returns the result map (throwing on failure), while `exec_stream` lets you process stdout/stderr in real time. |

#### Pinning the working directory with `dir()`

- Call `dir(path)` once inside each `task()` to lock the working directory for external commands and triggered tasks. Paths starting with `/` are treated as absolute; all other paths are resolved relative to the directory that contains `rhaskfile.rhai` (or the script passed via `-f/--file`) and canonicalized into an absolute path.
- Rhask validates the path when the script is loaded. Missing files or non-directory targets abort with an error such as `dir(): '...' is not a directory.`.
- When `dir()` is set, Rhask changes the process directory before running `actions()`. Every `exec(cmd([...]).build())` call and every triggered task runs from its own declared directory; parent settings are never inherited.
- Tasks without `dir()` continue to run inside the shell’s current working directory—the same behavior Rhask used before this helper existed. Use `dir(".")` or `dir("scripts")` to make the intent explicit.
- Relative paths always resolve against the directory that hosted the **first** `rhaskfile` you loaded (typically the root file passed to `-f/--file`). If you import a script from that root but later execute the same script standalone via `rhask -f child/file.rhai`, the base directory changes and existing `dir("relative/path")` entries may point somewhere else. Keep this limitation in mind when sharing scripts between standalone and imported use cases.

```rhai
task("coverage", || {
    description("Run coverage helper script from scripts/");
    dir("scripts");
    actions(|| {
        exec(cmd(["./coverage.sh", "--mode", "unit"]).build());
    });
});
```

#### Running external commands (`cmd` / `exec` / `exec_stream`)

1. **Describe the pipeline**  
   Use `cmd([program, arg, ...])` with one or more strings, chain `.pipe(cmd([...]))` for additional stages, and optionally hang `.env(#{ KEY: "VALUE" })`. Call `.build()` once the pipeline is ready, then tweak the executor with `.timeout(ms)` / `.allow_exit_codes([...])` as needed.

2. **Execute it safely**  
   - `exec(cmd(...).pipe(...).build())` mirrors stdout/stderr to the terminal and returns `#{ success, status, stdout, stderr, duration_ms }`. Non-zero exits (unless explicitly allowed) trigger a `throw`, so your task stops immediately.
   - `exec_stream(executor, stdout_cb?, stderr_cb?)` is the streaming variant. Omit the callbacks to show the output directly; the returned map will contain empty `stdout`/`stderr`.

3. **Run it inside `actions()`**  
   Pipelines can be assembled anywhere, but “firing” them via `exec()` / `exec_stream()` is only allowed inside `actions()`, which keeps `dir()` semantics intact.
   ```rhai
   actions(|| {
       let pipeline = cmd(["git", "branch", "-vv"])
           .pipe(cmd(["grep", "gone]"]))
           .pipe(cmd(["awk", "{print $1}"]))
           .build();

       let result = exec(pipeline);
       print("deleted branches:\n" + result.stdout);
   });
   ```

---

## Shell Completions

Generate completion scripts via `rhask completions <shell>` and source them from your shell configuration:

```bash
rhask completions bash > ~/.local/share/bash-completion/rhask
source ~/.local/share/bash-completion/rhask
```

Do the same for Zsh/Fish by placing the generated file under each shell’s completion directory. The completion script covers both CLI subcommands/options and dynamically defined tasks/groups. If you pass `-f/--file` to point Rhask at another `rhaskfile`, the completion function forwards that value so `TAB` still offers candidates from the correct script.

---

## Logs & Output

- User-facing messages are separated: informational lines go to `stdout`, warnings/errors to `stderr`.
- Logging relies on `env_logger`. Set `RUST_LOG=debug rhask run …` (or `trace`, etc.) when you need troubleshooting details.
- Color output is automatically enabled when stdout is a TTY and disabled for non-TTY contexts such as CI pipelines.

---

## Coverage

Rhask ships with a helper script for measuring code coverage. It installs `cargo-llvm-cov` / `llvm-tools-preview` on demand (requires `rustup`).

- `./scripts/coverage.sh --mode all` (default)  
  Runs unit + integration tests and writes `target/coverage/html/index.html`.
- `./scripts/coverage.sh --mode unit`  
  Runs unit tests only and writes `target/coverage-unit/html/index.html`.
- `./scripts/coverage.sh --mode integration`  
  Runs `tests/*.rs` only and writes `target/coverage-integration/html/index.html`.

Pass `-- <extra flags>` if you need to forward arguments directly to `cargo llvm-cov` (for example `-- --lcov --output-path lcov.info`). This script is safe to call from CI workflows as well.

---

## License

Dual-licensed under MIT OR Apache-2.0.  
See `LICENSE-MIT` and `LICENSE-APACHE` for details.
