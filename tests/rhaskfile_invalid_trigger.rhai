task("clean", || {
    actions(|| {
        print("should never run");
    });
});

trigger("clean");
