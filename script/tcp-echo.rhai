// tcp-echo.rhai — minimal concurrent TCP echo server.
//
// Usage:
//   recon --script tcp-echo [ADDR]     # ADDR default 127.0.0.1:9000
//
// Accepts connections in a loop. Each connection is handled on its
// own thread: read a line, echo it back, close. Ctrl-C to stop (or
// wrap in a --max-time envelope when driving from CI).

let addr = if args.len() > 1 { args[1] } else { "127.0.0.1:9000" };
print(`echo server listening on ${addr}`);

let l = tcp_listen(addr);

loop {
    // 30-second accept timeout so Ctrl-C can actually land between
    // accepts. Remove the timeout for a pure blocking accept.
    let conn = try_accept(l);
    if conn == () { continue; }

    thread_spawn(|c| {
        let peer = tcp_peer_addr(c);
        print(`  conn from ${peer}`);
        let line = tcp_read_line(c, 5000);
        print(`  got: ${line}`);
        tcp_write(c, `echo: ${line}` + "\n");
        tcp_close(c);
    }, conn);
}

fn try_accept(listener) {
    try {
        return tcp_accept(listener, 30000);
    } catch(e) {
        if e.contains("timeout") { return (); }
        throw e;
    }
}
