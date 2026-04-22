// Usage: recon --script rtsp [URL]
//
// RTSP OPTIONS probe. Prints status line + supported methods.

let url = if args.len() > 1 { args[1] } else { "rtsp://wowzaec2demo.streamlock.net/vod/mp4:BigBuckBunny_115k.mov" };
let r = rtsp(url);
print(`${r.host}:${r.port}  ${r.status_line}`);
if type_of(r.methods) == "array" && r.methods.len() > 0 {
    print(`  methods: ${r.methods}`);
}
return if r.status_code == 200 { 0 } else { 1 };
