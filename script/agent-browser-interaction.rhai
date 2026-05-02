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
agentBrowser::back();

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
agentBrowser::keyboard_type("Hello, world!");
agentBrowser::keyboard_insert("Inserted without key events");
try { agentBrowser::press("Tab"); } catch(e) { print(`press Tab: ${e}`); }
try { agentBrowser::press("Control+a"); } catch(e) { print(`press Ctrl+A: ${e}`); }

// ── Scroll ───────────────────────────────────────────────────────────────────
agentBrowser::scroll("down");           // 1 step in the named direction
agentBrowser::scroll("down", 600);      // 600 pixels
agentBrowser::scroll("up", 400);
try { agentBrowser::scrollintoview("footer"); } catch(e) { print(`scrollintoview: ${e}`); }

// ── Wait — accepts ms (string or int) or selector ────────────────────────────
agentBrowser::wait("500");                  // 500 ms
try { agentBrowser::wait("body"); } catch(e) { print(`wait selector: ${e}`); }

agentBrowser::close();
return 0;
