// thread.rhai — spawn + channel + join demo.
//
// Usage: recon --script thread
//
// Fan out three workers that each send a message through a channel.
// The main thread collects the results via recv with a timeout.

let c = channel();
let tx = c[0];
let rx = c[1];

// thread_spawn takes a FnPtr closure + optional arg (or array of args).
let handles = [];
for i in 0..3 {
    let h = thread_spawn(|n| {
        sleep(50 * n);  // stagger so order-of-completion is visible
        send(tx, `worker ${n} reporting tid=${tid()}`);
    }, i);
    handles.push(h);
}

// Wait on each spawn so we know it finished (join surfaces panics /
// errors that happened inside the closure).
for h in handles {
    join(h);
}

// Drain the channel.
for j in 0..3 {
    let msg = recv(rx, 1000);
    print(`  ← ${msg}`);
}

// Bounded channel demo: try_send returns false when full.
let b = channel_bounded(1);
let btx = b[0];
try_send(btx, "first");
let ok = try_send(btx, "second");
print(`bounded second try_send: ${ok}   (false = channel full)`);

return 0;
