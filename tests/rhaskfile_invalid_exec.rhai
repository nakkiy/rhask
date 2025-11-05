exec("echo should-not-run");

task("noop", || {
    actions(|| {
        print("this should never execute");
    });
});
