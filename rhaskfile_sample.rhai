group("sample", || {
    group("demo", || {
        description("Demo tasks");

        task("always_fail", || {
            description("Task that always fails");
            actions(|| {
                print("always_fail running...");
                exec(cmd(["false"]).build());
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
                trigger("sample.demo.always_fail");
                trigger("sample.demo.echo_success"); // should not run
            });
        });

        task("continue_on_error", || {
            description("Continue even if an error occurs");
            actions(|| {
                try {
                    trigger("sample.demo.always_fail");
                } catch {
                    print("always_fail failed but continuing");
                }
                trigger("sample.demo.echo_success");
            });
        });
    });

    group("docker", || {
        task("up", || {
            description("docker up");
            actions(|| {
                exec_stream(cmd(["docker", "run", "--rm", "-d", "--name", "demo", "alpine:3.20", "/bin/sh", "-c", "tail -f /dev/null"]).build());
            });
        });

        task("connect", || {
            description("docker connect");
            actions(|| {
                exec_stream(cmd(["docker", "exec", "-it", "demo", "/bin/sh"]).build());
            });
        });

        task("stop", || {
            description("docker stop");
            actions(|| {
                exec_stream(cmd(["docker", "stop", "demo"]).build());
            });
        });
    });
});
