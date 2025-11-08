task("fzf", || {
    description("choose and run a task via fzf (optionally scoped to a group)");
    args(#{
        group: ""
    });
    actions(|group| {
        let list_cmd = if group.is_empty() {
            "rhask list -F".to_string()
        } else {
            "rhask list -F ".to_string() + group
        };
        exec(
            list_cmd + " | fzf | awk '{print $1}' | xargs -r rhask run"
        );
    });
});

group("build_suite", || {
    description("Collection of build-related tasks");

    task("clean", || {
        description("Remove build artifacts");
        actions( || {
            exec("cargo clean");
            print("Cleanup completed");
        });
    });

    task("build", || {
        description("Build the project");
        actions( || {
            exec("cargo build");
            print("Build finished");
        });
    });

    task("test", || {
        description("Run tests");
        actions( || {
            exec("cargo test");
            print("Tests completed");
        });
    });

    task("fmt", || {
        description("Run rustfmt");
        actions( || {
            exec("cargo fmt");
            print("Formatting completed");
        });
    });

    task("dev", || {
        description("Execute common development tasks");
        actions( || {
            trigger("fmt");
            print("fmt finished");
            trigger("test");
            print("test finished");
            trigger("build");
            print("build finished");
            print("Development task collection finished");
        });
    });
});

import "rhaskfile_demo";
import "rhaskfile_sample";
