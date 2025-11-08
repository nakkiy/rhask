group("sample", || {
    group("demo", || {
        description("Demo tasks");

        task("always_fail", || {
            description("Task that always fails");
            actions(|| {
                print("always_fail running...");
                exec("false");
            });
        });

        task("echo_success", || {
            description("Task that succeeds");
            actions(|| {
                print("echo_success running...");
                print("Done");
            });
        });

        task("stop_on_error", || {
            description("Stop the flow on error");
            actions(|| {
                trigger("demo.always_fail");
                trigger("demo.echo_success"); // should not run
            });
        });

        task("continue_on_error", || {
            description("Continue even if an error occurs");
            actions(|| {
                try {
                    trigger("demo.always_fail");
                } catch {
                    print("always_fail failed but continuing");
                }
                trigger("demo.echo_success");
            });
        });
    });
});
