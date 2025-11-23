default_task("fzf");

task("fzf", || {
    description("choose and run a task via fzf (optionally scoped to a group)");
    args(#{
        group: ""
    });
    actions(|group| {
        let pipeline = if group.is_empty() {
            cmd(["rhask", "list", "-F"])
        } else {
            cmd(["rhask", "list", "-F", group])
        };
        exec(
            pipeline.pipe(cmd(["fzf"]))
                .pipe(cmd(["awk", "{print $1}"]))
                .pipe(cmd(["xargs", "-r", "rhask", "run"]))
                .build()
        );
    });
});

task("dev", || {
    description("Execute common development tasks. arg: fmt, test, clippy, build (default: all)");

    args(#{cmd: ""});
    actions(|cmd_value| {
        let allowed = ["fmt", "test", "clippy", "build"];
        let cmds = if cmd_value.is_empty() {
            allowed
        } else {
            if allowed.contains(cmd_value){
                [cmd_value]
            } else {
                throw("Unknown command: " + cmd_value);
            };
        };
        for item in cmds {
            print("Running cargo " + item + "...");
            exec(cmd(["cargo", item]).build());
            print("cargo " + item + " finished");
        };
        print("Development task collection finished");
    });
});

group("caverge", || {
    description("Code coverage tasks");

    let list = ["all", "unit", "integration"];
    for opt in list {
        let mode = opt;
        task(mode, || {
            description("Run coverage mode: " + mode);

            dir("./scripts");
            actions(|| {
                print("Running coverage with option: " + mode + "...");
                exec_stream(cmd(["./coverage.sh", "--mode", mode]).build());
                print("Coverage task finished");
            });
        });
    };
});

group("git", || {
    description("Helpers for pruning remote-tracking branches");

    task("preview", || {
        description("Show branches tracked as gone (no deletion)");
        actions(|| {
            exec(cmd(["git", "fetch", "--prune"]).build());
            exec(
                cmd(["git", "branch", "-vv"])
                    .pipe(cmd(["grep", "gone]"]))
                    .build()
            );
        });
    });

    task("clean", || {
        description("Delete local branches whose upstream is gone");
        actions(|| {
            exec(cmd(["git", "fetch", "--prune"]).build());
            exec(
                cmd(["git", "branch", "-vv"])
                    .pipe(cmd(["grep", "gone]"]))
                    .pipe(cmd(["awk", "{print $1}"]))
                    .pipe(cmd(["xargs", "git", "branch", "-D"]))
                    .build()
            );
        });
    });
});

import "rhaskfile_sample";