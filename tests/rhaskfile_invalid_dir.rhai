task("duplicate_dir", || {
    description("dir() called twice should fail during load");
    dir(".");
    dir(".");
    actions(|| {
        print("unreachable");
    });
});
