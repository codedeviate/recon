// tui.rhai — multi-pane dashboard with subprocess output routing.
// Usage: recon --script tui.rhai
//
// Demonstrates the pattern the feature was built for: a long-running
// shell command's output streams into one pane while the script's
// own progress messages go to another. No external commands required —
// the demo simulates the workload with a `seq + sleep` loop so it
// runs on any machine without dependencies.

tui::run(|d| {
    let parts = d.split_vertical([70, 30]);
    let main = parts[0];
    let status = parts[1];

    main.title("subprocess output");
    status.title("script progress");

    status.println("[1/3] starting first batch");
    shell_stream(
        "for i in 1 2 3 4 5; do echo \"batch-1 step $i\"; sleep 0.15; done",
        |line| main.println(line),
    );

    status.println("[2/3] running second batch");
    shell_stream(
        "for i in a b c d e; do echo \"batch-2 step $i\"; sleep 0.15; done",
        |line| main.println(line),
    );

    status.println("[3/3] finishing up");
    shell_stream("sleep 0.5 && echo 'final cleanup ok'", |line| main.println(line));

    status.println("done — dashboard will close in 1s");
    sleep_ms(1000);
});

print("dashboard closed cleanly");
()
