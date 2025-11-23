# Rhask – Rhai-based Task Runner

Rhask is a lightweight task runner written in Rust. Tasks and groups are authored in [Rhai](https://rhai.rs/), can be nested arbitrarily, and are invoked through fully qualified names such as `group.task` (or by short names when they are unique).

![demo](./demo.png)

---

## Quick Start (Install / Setup)

```bash
# Install from crates.io
cargo install rhask

# List tasks at the project root
rhask list

# Run a task
rhask run <task>
```

Example `rhaskfile.rhai`:

```rhai
task("hello", || {
    actions(|| {
        print("Hello from Rhask!");
    });
});
```

- Place `rhaskfile.rhai` at the repository root (copy `rhaskfile_demo.rhai` or `rhaskfile_sample.rhai` to get started).
- Use `-f/--file` to point Rhask at any Rhai script, e.g. `rhask -f ./rhaskfile_demo.rhai list`.
- `rhask run <task>` and `rhask <task>` behave the same; ambiguous short names print the candidates.
- Declare `default_task("group.task")` so `rhask` (with no arguments) runs that entry, otherwise it falls back to listing tasks.

---

## Features

### 1. File loading & project root discovery
- The configuration file is **`rhaskfile.rhai`**.
- Rhask searches the current directory and walks up the parents, loading **only the first file it finds**.
- Override the search path with `-f` / `--file` to load any other Rhai script.

---

### 2. Declaring and managing tasks/groups
- Use `task("name", || { ... })` / `group("name", || { ... })` to build nested hierarchies.
- Nest as deeply as you like; there is no limit on groups or tasks.
- Execute entries via fully qualified names such as `group.subgroup.task`.
- When a short name is unique you can omit the prefix; conflicts print a candidate list and require a fully qualified retry.

---

### 3. Listing tasks (tree vs. flat)
- You can scope the listing to a subtree (e.g. `rhask list deploy -F`).

#### Tree view
`rhask list`
- Shows the hierarchy as an indented tree.
- `description()` text is aligned on the right-hand side.

#### Flat view
`rhask list --flat` / `rhask list -F`
- Prints each task as `full.path  description` on a single line (colorized on TTYs).

---

### 4. Execution rules (run / default tasks / ambiguity)
- Execute tasks via `rhask run <task>` or the shorthand `rhask <task>`.
- Running bare `rhask` behaves as follows:
  - Execute the task registered via `default_task("...")` when present.
  - Otherwise fall back to `rhask list`.
- Ambiguous `<task>` names **print candidates and exit** (Rhask will not guess). Re-run with the full path.
- `args(#{ key: default, ... })` declares CLI parameters; `()` marks them as required.
  - CLI values may be passed as positional arguments, `key=value`, `--key=value`, or `--key value`, and you can mix the styles.

---

### 5. Working directories with `dir()`
- Call `dir("path")` **once per task** to pin its working directory.
- Relative paths are resolved from the directory that hosts `rhaskfile.rhai`; absolute paths are left unchanged.
- Paths are validated at load time—nonexistent or non-directory paths raise an error.
- When a task `trigger()`s another task, the callee’s own `dir()` always wins; parent settings are never inherited.
- Without `dir()` the task runs in the shell directory where you launched `rhask`.

---

### 6. Shell-free external commands (`cmd` / `pipe` / `exec`)

#### 1. `cmd()`
```rhai
cmd(["git", "status"])
```
- Describes a command as an array, avoiding shell quoting issues.

#### 2. `pipe()`
```rhai
cmd(["git", "branch", "-vv"])
    .pipe(cmd(["grep", "gone"]))
```
- Chains any number of stages; each becomes a native process.
- Lets you express `git | grep | awk`-style flows without shell syntax.

#### 3. `build()`
- Finalizes the pipeline before execution.
- Tweak behavior via `.timeout(ms)`, `.env(#{})`, `.allow_exit_codes([0, 1])`, and similar helpers.

#### 4. `exec()`
- Mirrors stdout/stderr to the console and returns `#{ success, status, stdout, stderr, duration_ms }`.
- Throws when the exit code is not allowed.

#### 5. `exec_stream()`
- Streaming-friendly variant that lets you process stdout/stderr via callbacks (omit them to stream directly to the terminal).

---

### 7. Other utilities
- `rhask completions <shell>` generates Bash/Zsh/Fish completion scripts (task names included).
- The repo bundles `scripts/coverage.sh` as a helper around `cargo llvm-cov`.
- Rhai `import` statements work as in upstream Rhai, so you can split large task files as needed.

---

## Usage

### CLI Commands

| Command | Description |
| --- | --- |
| `rhask list [group]` | Display registered tasks/groups as a tree. Passing a fully qualified name limits the output to that subtree. |
| `rhask list --flat` / `rhask list -F` | Print each task as `full.path` plus an aligned description (colorized on TTYs, works with `group` filters and tools like `fzf`). |
| `rhask run <task> [args…]` | Execute a task. Ambiguous leaves print the candidates and ask you to re-run with a full path. The shorthand `rhask <task>` behaves the same. |
| `rhask -f <file> …` | Explicitly load a Rhai script. Place `-f/--file` before the subcommand or task name (e.g. `rhask -f ./demo.rhai list`). |
| `rhask` (no arguments) | Run the configured `default_task()` or fall back to `rhask list` when unset. |
| `rhask completions <shell>` | Emit shell completion scripts (see below). |

### Passing Arguments

Declaring `args(#{ target: (), profile: "debug" })` assigns positional arguments in lexicographic key order (`profile` → `target` in this example). `()` marks required parameters. Use whichever CLI style you prefer:

- Positional: `rhask run build release x86_64-unknown-linux-gnu`
- `key=value`: `rhask run build profile=release target=x86_64-unknown-linux-gnu`
- `--key=value`: `rhask run build --target=x86_64-apple-darwin`
- `--key value`: `rhask run build --target wasm32-unknown-unknown`
- Mix and match as needed

Unknown keys raise an error, and missing required values trigger a descriptive failure.

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

### Helpers Available from Rhai

| Function | Description |
| --- | --- |
| `task(name, \|\| { ... })` | Declare a task; call `description`/`actions`/`args` inside it. |
| `group(name, \|\| { ... })` | Declare a group; nest additional groups or tasks. |
| `description(text)` | Usable inside `task()`/`group()`; sets the label shown in listings (call once per task). |
| `actions(\|\| { ... })` | Usable inside `task()`; registers the executable closure (call once). Invoke `trigger()` or `exec(...)` from here. |
| `args(#{ key1: default1, key2: (), ... })` | Usable inside `task()`; declares CLI parameters. `()` = no default = required. Call once per task. |
| `dir(path)` | Usable inside `task()`; pins the working directory (call once). Relative paths resolve from the rhaskfile directory; absolute paths stay as-is. Invalid paths error at load time. |
| `default_task("full.path")` | Declare once at the top level (imports included) to define the fallback when `rhask` is run without arguments. |
| `trigger(name, positional?, named?)` | Usable inside `actions()`; runs another task. Accepts positional arrays and/or named maps. The callee’s `dir()` takes precedence over the caller’s. |
| `cmd([cmd, arg, ...])` | Build external commands inside `actions()`. Chain `.env()` / `.pipe()` and finish with `.build()` before running `exec(...)` or `exec_stream(...)`. |
| `exec(pipeline)` / `exec_stream(pipeline, stdout_cb?, stderr_cb?)` | Usable inside `actions()`; execute pipelines and receive `#{ success, status, stdout, stderr, duration_ms }`. `exec_stream` lets you process output live. |

#### Pinning the working directory with `dir()`

- `dir(path)` is allowed once per `task()`. Absolute paths remain untouched; relative paths are resolved against the directory that hosts the loaded rhaskfile (the one Rhask found or the file passed via `-f/--file`).
- Nonexistent paths or non-directories abort loading with errors such as `dir(): '...' is not a directory.`
- When `dir()` is present, Rhask `chdir`s into that location before executing `actions()`. Nested `exec()`/`trigger()` calls always honor the callee’s `dir()` rather than inheriting from parents.
- Tasks without `dir()` run in the shell directory from which you launched `rhask`. Set `dir(".")` or `dir("scripts")` explicitly if you need predictability.
- The resolution root is always the directory of the initially loaded rhaskfile. If you run a child script directly via `rhask -f child/file.rhai`, relative paths will resolve from that child file instead, so plan accordingly.

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
   Build it with `cmd([program, arg, ...])`, chain `.pipe(cmd([...]))`, override the environment via `.env(#{ KEY: "VALUE" })`, and call `.build()`. After `build()` you can still attach `.timeout(ms)` or `.allow_exit_codes([0, 1])`.
2. **Execute it**  
   - `exec(cmd(...).pipe(...).build())` returns `#{ success, status, stdout, stderr, duration_ms }`, mirrors stdout/stderr to the terminal, and throws on disallowed exit codes.
   - `exec_stream(cmd(...).build(), stdout_cb?, stderr_cb?)` suits streaming workloads. Omit the callbacks to stream directly to the console.
3. **Run inside `actions()`**  
   Pipelines can be assembled anywhere, but actually executing them is restricted to `actions()` so `dir()` semantics and nested `trigger()` calls stay consistent.

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

```bash
rhask completions bash > ~/.local/share/bash-completion/rhask
echo "source ~/.local/share/bash-completion/rhask" >> ~/.bashrc
```

Drop the generated file into the appropriate completion directory (Bash/Zsh/Fish) and source it. Task/group names defined in Rhai are part of the completion results. When you pass `-f/--file`, the completion function forwards that value so suggestions always match the referenced rhaskfile.

---

## Logs & Output

- User-facing information goes to `stdout`; warnings and errors go to `stderr`.
- Enable tracing with `RUST_LOG=debug rhask run …` (or `trace`) thanks to the `env_logger` backend.
- Calls such as `trigger()` or `exec(cmd([...]).build())` must happen inside `actions()`; doing so elsewhere raises errors.
- Color output is enabled automatically on TTYs and disabled for redirected/CI environments.

---

## Coverage

`scripts/coverage.sh` wraps `cargo-llvm-cov` / `llvm-tools-preview`, installing the tools on demand (requires `rustup`).

- `./scripts/coverage.sh --mode all` (default):  
  run unit + integration tests and write `target/coverage/html/index.html`.
- `./scripts/coverage.sh --mode unit`:  
  run unit tests only and write `target/coverage-unit/html/index.html`.
- `./scripts/coverage.sh --mode integration`:  
  run `tests/*.rs` only and write `target/coverage-integration/html/index.html`.

Arguments after `--` are forwarded directly to `cargo llvm-cov`. Use the same script from CI as well.

---

## License

Dual-licensed under MIT OR Apache-2.0.

---
