fn log_task(name, message) {
    print("[" + name + "] " + message);
}

fn log_build(profile, target) {
    let detail = "profile=" + profile + ", target=" + target;
    log_task("build", detail);
}

default_task("clean");

task("clean", || {
    description("Remove build artifacts");
    actions(|| {
        log_task("clean", "workspace cleaned");
    });
});

task("build", || {
    description("Build with profile and target");
    args(#{
        profile: "debug",
        target: "x86_64-unknown-linux-gnu"
    });
    actions(|profile, target| {
        log_build(profile, target);
    });
});

task("trigger_build_release", || {
    description("Trigger build with a release profile");
    actions(|| {
        trigger("build", ["release"]);
    });
});

task("trigger_build_named", || {
    description("Trigger build with named arguments");
    actions(|| {
        trigger("build", #{ target: "wasm32-unknown-unknown" });
    });
});

task("trigger_build_mixed", || {
    description("Trigger build mixing positional and named args");
    actions(|| {
        trigger(
            "build",
            ["release"],
            #{ target: "x86_64-apple-darwin" }
        );
    });
});

task("requires_version", || {
    description("Version argument is required");
    args(#{ version: () });
    actions(|version| {
        log_task("requires_version", "version=" + version);
    });
});

task("trigger_clean", || {
    description("Reuse clean task through trigger()");
    actions(|| {
        log_task("trigger_clean", "delegating to clean");
        trigger("clean");
    });
});

task("exec_echo", || {
    description("Run echo through exec()");
    actions(|| {
        log_task("exec_echo", "spawning echo helper");
        exec(cmd(["echo", "fixture-exec-output"]).build());
    });
});

task("no_actions", || {
    description("Task intentionally missing actions()");
    // intentionally empty
});

task("trigger_unknown", || {
    description("Trigger an unknown task");
    actions(|| {
        trigger("unknown_task_for_tests");
    });
});

group("build_suite", || {
    description("Collection of build-related tasks");

    task("build_debug", || {
        description("Run a debug build");
        actions(|| {
            log_task("build_debug", "pipeline started");
            trigger("build");
        });
    });

    group("release_flow", || {
        description("Additional steps for releases");

        task("package_artifacts", || {
            description("Package the build artifacts");
            actions(|| {
                log_task("package_artifacts", "archive ready");
            });
        });

        task("deploy_staging", || {
            description("Deploy to staging");
            actions(|| {
                log_task("deploy_staging", "deploy to staging (fixture)");
            });
        });
    });
});

group("ops_suite", || {
    description("Operations task bundle");

    task("deploy_staging", || {
        description("Deploy to staging (operations version)");
        actions(|| {
            log_task("ops.deploy_staging", "operations deploy");
        });
    });
});

group("ops_release_flow", || {
    description("Operations release workflow");

    task("monitor", || {
        description("Start monitoring");
        actions(|| {
            log_task("monitor", "ops monitor started");
        });
    });
});

group("build_ops_release_flow", || {
    description("Example with overlapping group names");

    group("release_flow", || {
        description("Dummy release flow");
        task("noop", || {
            description("No-op task");
            actions(|| {
                log_task("noop", "fixture noop");
            });
        });
    });
});
