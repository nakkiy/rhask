group("greet", || {
    description("Greeting tasks");

    task("rust", || {
        description("Say hello to Rust with print");
        actions(|| {
            print("Hello, Rust!");
        });
    });
    task("rhai", || {
        description("Say hello to Rhai via an external command");
        actions(|| {
            exec(cmd(["echo", "Hello, Rhai!"]).build());
        });
    });
    task("all", || {
        description("Say hello to all with triggers");
        actions(|| {
            trigger("rust");
            trigger("rhai");
            print("Hello, Rhask!");
        });
    });
});
