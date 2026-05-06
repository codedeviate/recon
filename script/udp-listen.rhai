// udp-listen.rhai — minimal UDP beacon listener.
//
// Usage:
//   recon --script udp-listen [ADDR]    # ADDR default 127.0.0.1:9001
//
// Binds and prints each incoming datagram's source + payload. Ctrl-C
// to stop. Test from another shell:
//   echo "hello" | nc -u 127.0.0.1 9001

let addr = if args.len() > 1 { args[1] } else { "127.0.0.1:9001" };
print(`udp listener on ${addr}`);

let s = udp_bind(addr);

loop {
    let r = try_recv(s);
    if r == () { continue; }
    print(`  ${r.addr} → ${r.data.len()} bytes`);
}

fn try_recv(sock) {
    try {
        return udp_recv_from(sock, 65536, 30000);
    } catch(e) {
        // EAGAIN ("Resource temporarily unavailable", os error 35 on
        // macOS / 11 on Linux) is the would-block path the OS returns
        // when the socket is non-blocking and no datagram is queued.
        // Treat it the same as a timeout.
        if e.contains("timed out")
            || e.contains("timeout")
            || e.contains("Resource temporarily unavailable")
            || e.contains("os error 35")
            || e.contains("os error 11")
        { return (); }
        throw e;
    }
}
