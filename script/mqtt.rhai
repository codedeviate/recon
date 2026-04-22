// Usage: recon --script mqtt [URL] [PAYLOAD]
//
// MQTT publish with guards on broker reachability.

let url = if args.len() > 1 { args[1] } else { "mqtt://test.mosquitto.org/recon/script-demo" };
let payload = if args.len() > 2 { args[2] } else { "hello from recon" };
// Very light reachability check against the default port.
let host_port = url.replace("mqtt://", "").split("/").shift();
let default_port = if url.starts_with("mqtts://") { 8883 } else { 1883 };
if !host_port.contains(":") { host_port += `:${default_port}`; }
let t = tcp(`tcp://${host_port}`);
if !t.ok {
    print(`mqtt broker ${host_port} unreachable — skipping`);
    return 2;
}
let r = mqtt_pub(url, payload);
print(`published to ${url} in ${r.duration_ms}ms`);
return 0;
