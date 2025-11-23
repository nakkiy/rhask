exec(cmd(["echo", "should-not-run"]).build());

task("noop", || {
    actions(|| {
        print("this should never execute");
    });
});
