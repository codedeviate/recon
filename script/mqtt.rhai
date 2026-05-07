// Usage: recon --script mqtt [URL] [PAYLOAD]
//
// MQTT publish with guards on broker reachability. Exercises MQTT 5
// publish properties (user_properties + content_type) via the opts map
// — silently ignored on brokers that only speak MQTT 3.1.1.

let url = if args.len() > 1 { args[1] } else { "mqtt://test.mosquitto.org/recon/script-demo" };
let payload = if args.len() > 2 { args[2] } else { `{"from":"recon","ts":${now()}}` };

// Very light reachability check against the default port.
let stripped = if url.starts_with("mqtts://") {
    url.sub_string(8)
} else {
    url.sub_string(7)
};
let host_port = stripped.split("/").shift();
let default_port = if url.starts_with("mqtts://") { 8883 } else { 1883 };
if !host_port.contains(":") { host_port += `:${default_port}`; }
// `tcp()` raises a Rhai exception on connect failure / timeout, so a
// plain `if !t.ok` guard never fires — wrap it in try/catch and treat
// any failure (refused, DNS, timeout) as "broker unreachable".
let reachable = false;
try {
    let t = tcp(`tcp://${host_port}`, #{ timeout: 5 });
    reachable = t.ok;
} catch(e) {
    reachable = false;
}
if !reachable {
    print(`mqtt broker ${host_port} unreachable — skipping`);
    return 2;
}

// MQTT 5 properties on the PUBLISH packet: content_type +
// user-properties. Brokers expose these to subscribers via
// `mosquitto_sub -v -F '%J'` or similar.
let r = mqtt_pub(url, payload, #{
    content_type: "application/json",
    user_properties: [
        #{ key: "env",    value: "demo" },
        #{ key: "caller", value: "recon" },
    ],
});
print(`published to ${url} in ${r.duration_ms}ms`);
return 0;
