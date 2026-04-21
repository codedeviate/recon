import "agentBrowser" as agentBrowser;
let r = ping("www.example.com", 1);
if r.loss_pct == 0 {
    print("Works");
} else {
    print("Fails");
}
