// Demo: configure agent-browser global options for a script session.
//
// Common workflow: a target server has a self-signed cert (typical for
// staging / internal tools). Set the option once at script start and
// every subsequent verb inherits it.

if !agentBrowser::available {
    print("agent-browser not installed; skipping demo");
    return;
}

// Module-level defaults — applied to every verb in this script.
agentBrowser::set_default_options(#{
    ignore_https_errors: true,
    user_agent: "Recon/0.75 demo",
});

// Inspect what's set.
let current = agentBrowser::default_options();
print("Defaults: " + current);

// Per-call override on a launch verb (without disturbing module defaults).
let r = agentBrowser::open(
    "https://self-signed.badssl.com",
    #{ user_agent: "PerCall/UA" },
);
print(r);

// Headers as a Rhai map (auto-serialized to JSON).
agentBrowser::set_default_options(#{
    ignore_https_errors: true,
    headers: #{ Authorization: "Bearer test", "X-Custom": "value" },
});

// Repeatable extension flag.
agentBrowser::set_default_options(#{
    extension: ["/tmp/ext1", "/tmp/ext2"],
});

agentBrowser::clear_default_options();
print("Defaults cleared.");
