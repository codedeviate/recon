// Usage: recon --script rtsp [URL]
//
// RTSP OPTIONS probe. Prints status line + supported methods.
//
// Public demo endpoints (try these if the default is down):
//   rtsp://wowzaec2demo.streamlock.net/vod/mp4:BigBuckBunny_115k.mov
//     — Wowza Streaming Engine demo, long-running but being wound down.
//   rtsp://rtsp.stream/pattern
//     — rtsp.stream free color-bar test pattern (no auth).
//   rtsp://rtsp.stream/movie
//     — rtsp.stream free Big Buck Bunny clip (no auth).
//   rtsp://zephyr.rtsp.stream/movie?streamKey=feedc0de
//     — rtsp.stream demo via zephyr edge, stream-keyed.
//   rtsp://demo:demo@ipvmdemo.dyndns.org:5541/onvif-media/media.amp
//     — IPVM ONVIF camera demo; auth `demo`/`demo`, intermittent.

let url = if args.len() > 1 { args[1] } else { "rtsp://wowzaec2demo.streamlock.net/vod/mp4:BigBuckBunny_115k.mov" };

// Pre-flight TCP/554 probe — public RTSP demos come and go, and
// `rtsp()` raises on connect failure instead of returning a result
// with `ok=false`. Skip cleanly when the host is unreachable rather
// than crashing the demo with a noisy stack trace. See header
// comments for alternate hosts when the default is down.
let probe_target = url;
let i = probe_target.index_of("://");
if i >= 0 { probe_target = probe_target.sub_string(i + 3); }
let at = probe_target.index_of("@");
if at >= 0 { probe_target = probe_target.sub_string(at + 1); }
let slash = probe_target.index_of("/");
if slash >= 0 { probe_target = probe_target.sub_string(0, slash); }
if probe_target.index_of(":") < 0 { probe_target += ":554"; }
let reachable = false;
try {
    let t = tcp(`tcp://${probe_target}`, #{ timeout: 5 });
    reachable = t.ok;
} catch(e) { reachable = false; }
if !reachable {
    print(`rtsp host ${probe_target} unreachable — skipping (try a host from the script header)`);
    return 2;
}

let r = rtsp(url);
print(`${r.host}:${r.port}  ${r.status_line}`);
if type_of(r.methods) == "array" && r.methods.len() > 0 {
    print(`  methods: ${r.methods}`);
}
return if r.status_code == 200 { 0 } else { 1 };
