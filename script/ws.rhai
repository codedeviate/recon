// Usage: recon --script ws [URL]
//
// WebSocket handshake + Ping/Pong round-trip.

let url = if args.len() > 1 { args[1] } else { "wss://echo.websocket.org" };
let r = wss(url);
print(`${r.scheme}://${r.host}:${r.port}  status ${r.http_status}`);
print(`  connect=${r.connect_ms}ms handshake=${r.handshake_ms}ms ping=${r.ping_ms}ms`);
return if r.pong_nonce_matched { 0 } else { 1 };
