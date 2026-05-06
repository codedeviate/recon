// Usage: recon --script agent-browser-interaction [URL]
//
// Element interaction surface: click / dblclick / hover / focus / check /
// uncheck / fill / type_text / press / scroll / scrollintoview /
// keyboard_type / keyboard_insert. Selectors may be CSS, XPath, or refs
// from a prior snapshot (`@e3`).

if !agentBrowser::available {
    print("agent-browser not installed; skipping");
    return 2;
}

let url = if args.len() > 1 { args[1] } else { "https://example.com" };
agentBrowser::open(url);

// ── Mouse-style ──────────────────────────────────────────────────────────────
try { agentBrowser::click("a"); print("click ok"); } catch(e) { print(`click: ${e}`); }

// The click above almost certainly navigated. Re-open the URL so the
// rest of the demo runs on a known-good page. Bare `back()` would throw
// "Inspected target navigated or closed" on a fully-navigated target.
try { agentBrowser::open(url); } catch(e) { print(`re-open after click: ${e}`); }

try { agentBrowser::dblclick("h1"); print("dblclick ok"); } catch(e) { print(`dblclick: ${e}`); }
try { agentBrowser::hover("a"); print("hover ok"); } catch(e) { print(`hover: ${e}`); }
try { agentBrowser::focus("a"); print("focus ok"); } catch(e) { print(`focus: ${e}`); }

// ── Form fields ──────────────────────────────────────────────────────────────
// `fill` clears then sets value; `type_text` keystrokes character-by-character
// (slower but triggers JS keystroke handlers).
try { agentBrowser::fill("input[name='q']", "recon"); } catch(e) { print(`fill: ${e}`); }
try { agentBrowser::type_text("input[name='q']", "more"); } catch(e) { print(`type_text: ${e}`); }

// ── Checkboxes ───────────────────────────────────────────────────────────────
try { agentBrowser::check("input[type='checkbox']"); } catch(e) { print(`check: ${e}`); }
try { agentBrowser::uncheck("input[type='checkbox']"); } catch(e) { print(`uncheck: ${e}`); }

// ── Keyboard (no selector — types into the focused element) ──────────────────
// Wrap each bare call in try/catch — heavy SPAs (e.g. news sites) may
// trigger mid-action navigation that puts the inspected target in a
// transient bad state.
try { agentBrowser::keyboard_type("Hello, world!"); } catch(e) { print(`keyboard_type: ${e}`); }
try { agentBrowser::keyboard_insert("Inserted without key events"); } catch(e) { print(`keyboard_insert: ${e}`); }
try { agentBrowser::press("Tab"); } catch(e) { print(`press Tab: ${e}`); }
try { agentBrowser::press("Control+a"); } catch(e) { print(`press Ctrl+A: ${e}`); }

// ── Scroll ───────────────────────────────────────────────────────────────────
try { agentBrowser::scroll("down"); } catch(e) { print(`scroll down 1: ${e}`); }
try { agentBrowser::scroll("down", 600); } catch(e) { print(`scroll down 600: ${e}`); }
try { agentBrowser::scroll("up", 400); } catch(e) { print(`scroll up 400: ${e}`); }
try { agentBrowser::scrollintoview("footer"); } catch(e) { print(`scrollintoview: ${e}`); }

// ── Wait — accepts ms (string or int) or selector ────────────────────────────
try { agentBrowser::wait("500"); } catch(e) { print(`wait 500: ${e}`); }
try { agentBrowser::wait("body"); } catch(e) { print(`wait selector: ${e}`); }

// `close()` is best-effort — the target may already be gone.
try { agentBrowser::close(); } catch(e) { /* already closed */ }
return 0;
