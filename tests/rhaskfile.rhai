task("clean", || {
    description("Remove build artifacts");
    actions(|| {
        print("Cleanup completed");
    });
});

task("build", || {
    description("Build with profile and target");
    args(#{
        profile: "debug",
        target: "x86_64-unknown-linux-gnu"
    });
    actions(|profile, target| {
        print("build => profile:" + profile + ", target:" + target);
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
        print("requires_version => " + version);
    });
});

task("trigger_clean", || {
    description("Reuse clean task through trigger()");
    actions(|| {
        trigger("clean");
    });
});

task("loop", || {
    description("Print a simple counter loop");
    actions(|| {
        for n in 1..5 {
            print("count: " + n);
        }
    });
});

task("exec_echo", || {
    description("Run echo through exec()");
    actions(|| {
        exec("echo exec-from-fixture");
    });
});

group("build_suite", || {
    description("Collection of build-related tasks");

    task("build_debug", || {
        description("Run a debug build");
        actions(|| {
            print("debug build (test fixture)");
        });
    });

    group("release_flow", || {
        description("Additional steps for releases");

        task("package_artifacts", || {
            description("Package the build artifacts");
            actions(|| {
                print("package artifacts (test fixture)");
            });
        });

        task("deploy_staging", || {
            description("Deploy to staging");
            actions(|| {
                print("deploy to staging (fixture)");
            });
        });
    });
});

group("ops_suite", || {
    description("Operations task bundle");

    task("deploy_staging", || {
        description("Deploy to staging (operations version)");
        actions(|| {
            print("ops deploy (fixture)");
        });
    });
});

group("ops_release_flow", || {
    description("Operations release workflow");

    task("monitor", || {
        description("Start monitoring");
        actions(|| {
            print("ops monitor (fixture)");
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
                print("noop (fixture)");
            });
        });
    });
});
