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

import "samplerhaskfile";
