// Usage: recon --script unix-socket [SOCKET_PATH]
//
// Probe the Docker socket (or any UDS-exposed HTTP API) via the http()
// script binding's opts.unix_socket. Missing-socket errors bubble up
// naturally; the script uses sqlite() to read the socket's path from
// args before the probe so users can adapt it easily.

let socket = if args.len() > 1 { args[1] } else { "/var/run/docker.sock" };

// The http() call raises an exception if the socket doesn't exist; the
// script terminates with a non-zero exit in that case. Wrap in a
// function call if you want to catch gracefully from a parent script.
let r = http("http://localhost/_ping", #{ unix_socket: socket });

print(`${socket}: ${r.status} (${r.body_bytes.len()} bytes)`);
if r.headers["server"] != () {
    print(`server: ${r.headers["server"][0]}`);
}
if r.status >= 200 && r.status < 400 {
    print(`body: ${r.body}`);
    return 0;
}
return 1;
