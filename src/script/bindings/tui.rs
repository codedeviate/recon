//! `tui::run(callback)` — a minimal multi-pane dashboard for scripts.
//!
//! Designed to sit on top of `shell_stream`: a script can launch one
//! or more long-running subprocesses and route their output into
//! distinct text regions, while a status pane shows the script's own
//! progress messages.
//!
//! ```rhai
//! tui::run(|d| {
//!     let parts = d.split_vertical([60, 40]);
//!     let main = parts[0];
//!     let status = parts[1];
//!     main.title("subprocess output");
//!     status.title("progress");
//!
//!     status.println("starting brew upgrade");
//!     shell_stream("brew upgrade", |line| main.println(line));
//!     status.println("all done");
//! });
//! ```
//!
//! Architecture: a renderer thread owns the terminal and consumes
//! `PaneUpdate`s from an mpsc. The script-visible `Dashboard` and
//! `PaneHandle` types are thin senders. The closure runs synchronously
//! on the script's own thread — pane methods just push messages — so
//! `shell_stream`'s callback dispatch stays on a single thread and
//! captured-scope semantics match a normal loop.
//!
//! v1 limitations (intentionally deferred):
//!   - No raw mode: terminal resize during a redraw cycle may glitch
//!     until the next pane update triggers a fresh frame.
//!   - No PTY allocation: subprocesses that prompt for input or
//!     detect "not a TTY" still do so when piped through shell_stream.
//!   - Lines longer than the pane width are truncated, not wrapped.
//!   - Wide characters (emoji, CJK) count as one column even though
//!     terminals render them as two.

use crate::script::convert::err;
use crossterm::tty::IsTty;
use crossterm::{cursor, execute, queue, style::Print, terminal};
use rhai::{Array, Dynamic, Engine, EvalAltResult, FnPtr, NativeCallContext};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Process-wide flag: only one dashboard can be active at a time.
/// Nested `tui::run` calls would fight for the terminal.
static TUI_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn register(engine: &mut Engine) {
    let mut module = rhai::Module::new();
    let _ = module.set_native_fn(
        "run",
        |ctx: NativeCallContext, callback: FnPtr| -> Result<(), Box<EvalAltResult>> {
            run_dashboard(&ctx, callback)
        },
    );
    engine.register_static_module("tui", module.into());

    engine.register_type_with_name::<Dashboard>("Dashboard");
    engine.register_type_with_name::<PaneHandle>("PaneHandle");

    engine.register_fn(
        "split_vertical",
        |d: &mut Dashboard, percents: Array| -> Result<Array, Box<EvalAltResult>> {
            d.split(LayoutKind::Vertical, percents)
        },
    );
    engine.register_fn(
        "split_horizontal",
        |d: &mut Dashboard, percents: Array| -> Result<Array, Box<EvalAltResult>> {
            d.split(LayoutKind::Horizontal, percents)
        },
    );

    engine.register_fn("println", |p: &mut PaneHandle, line: &str| {
        p.send(PaneUpdate::Push(p.idx, line.to_string()));
    });
    engine.register_fn("title", |p: &mut PaneHandle, t: &str| {
        p.send(PaneUpdate::Title(p.idx, t.to_string()));
    });
    engine.register_fn("clear", |p: &mut PaneHandle| {
        p.send(PaneUpdate::Clear(p.idx));
    });
}

// ---------------------------------------------------------------------
// Script-visible types
// ---------------------------------------------------------------------

#[derive(Clone)]
struct PaneHandle {
    idx: usize,
    tx: mpsc::Sender<PaneUpdate>,
}

impl PaneHandle {
    fn send(&self, u: PaneUpdate) {
        let _ = self.tx.send(u);
    }
}

#[derive(Clone)]
struct Dashboard {
    tx: mpsc::Sender<PaneUpdate>,
    layout_set: Arc<AtomicBool>,
}

impl Dashboard {
    fn split(
        &self,
        kind: LayoutKind,
        percents: Array,
    ) -> Result<Array, Box<EvalAltResult>> {
        if self.layout_set.swap(true, Ordering::SeqCst) {
            return Err(err(
                "tui: split_vertical / split_horizontal can only be called once per dashboard",
            ));
        }
        if percents.is_empty() {
            return Err(err("tui: split needs at least one percent"));
        }
        let mut ps: Vec<u8> = Vec::with_capacity(percents.len());
        for (i, d) in percents.iter().enumerate() {
            let v = d
                .as_int()
                .map_err(|_| err(format!("tui: percent[{i}] must be an integer")))?;
            if v <= 0 || v > 100 {
                return Err(err(format!(
                    "tui: percent[{i}] = {v} out of (0, 100]"
                )));
            }
            ps.push(v as u8);
        }
        let n = ps.len();
        let _ = self.tx.send(PaneUpdate::Layout(kind, ps));
        let mut out = Array::with_capacity(n);
        for i in 0..n {
            out.push(Dynamic::from(PaneHandle {
                idx: i,
                tx: self.tx.clone(),
            }));
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
enum LayoutKind {
    #[default]
    Vertical,
    Horizontal,
}

enum PaneUpdate {
    Layout(LayoutKind, Vec<u8>),
    Push(usize, String),
    Title(usize, String),
    Clear(usize),
    Shutdown,
}

fn run_dashboard(
    ctx: &NativeCallContext,
    callback: FnPtr,
) -> Result<(), Box<EvalAltResult>> {
    // Refuse non-TTY stdout — a "dashboard" piped to a file is at best
    // a stream of ANSI codes nobody will look at.
    if !io::stdout().is_tty() {
        return Err(err(
            "tui::run: stdout is not a TTY (piped or redirected)",
        ));
    }

    if TUI_ACTIVE.swap(true, Ordering::SeqCst) {
        return Err(err(
            "tui::run: another dashboard is already active in this process",
        ));
    }

    let (tx, rx) = mpsc::channel::<PaneUpdate>();

    // Drop guard: restores the terminal on early-return, panic, or
    // normal exit. Symmetry is the whole point — anything that would
    // strand the user in alt-screen is a bug.
    struct RestoreGuard;
    impl Drop for RestoreGuard {
        fn drop(&mut self) {
            let _ = execute!(
                io::stdout(),
                terminal::LeaveAlternateScreen,
                cursor::Show
            );
            TUI_ACTIVE.store(false, Ordering::SeqCst);
        }
    }

    if let Err(e) = execute!(
        io::stdout(),
        terminal::EnterAlternateScreen,
        cursor::Hide
    ) {
        TUI_ACTIVE.store(false, Ordering::SeqCst);
        return Err(err(format!("tui::run: enter alt screen: {e}")));
    }
    let _guard = RestoreGuard;

    // Best-effort Ctrl-C handler: write the leave-alt-screen escape
    // and exit. `try_set_handler` returns Err if a handler is already
    // installed (the mqtt subscribe path uses one) — accept that and
    // continue; the Drop guard still fires for any non-signal exit.
    let _ = ctrlc::try_set_handler(|| {
        let _ = execute!(
            io::stdout(),
            terminal::LeaveAlternateScreen,
            cursor::Show
        );
        std::process::exit(130);
    });

    let renderer = thread::spawn(move || renderer_loop(rx));

    let dashboard = Dashboard {
        tx: tx.clone(),
        layout_set: Arc::new(AtomicBool::new(false)),
    };
    let dyn_d: Dynamic = Dynamic::from(dashboard);
    let result = callback.call_within_context::<Dynamic>(ctx, (dyn_d,));

    // Signal the renderer to stop and wait so we don't race against
    // its final redraw before the guard restores the screen.
    let _ = tx.send(PaneUpdate::Shutdown);
    let _ = renderer.join();
    let _ = io::stdout().flush();

    result.map(|_| ())
}

fn renderer_loop(rx: mpsc::Receiver<PaneUpdate>) {
    let mut state = RendererState::default();

    loop {
        match rx.recv_timeout(Duration::from_millis(150)) {
            Ok(PaneUpdate::Shutdown) => return,
            Ok(PaneUpdate::Layout(kind, percents)) => {
                state.kind = kind;
                state.percents = percents.clone();
                state.panes = (0..percents.len()).map(|_| PaneState::default()).collect();
                let _ = state.redraw();
            }
            Ok(PaneUpdate::Push(i, line)) => {
                if let Some(p) = state.panes.get_mut(i) {
                    p.lines.push(line);
                    // Cap the per-pane backlog so a 100k-line `find /`
                    // doesn't grow unbounded. 1000 lines comfortably
                    // outlasts any visible viewport on a sane terminal.
                    const MAX: usize = 1000;
                    if p.lines.len() > MAX {
                        let trim = p.lines.len() - MAX;
                        p.lines.drain(..trim);
                    }
                }
                let _ = state.redraw();
            }
            Ok(PaneUpdate::Title(i, t)) => {
                if let Some(p) = state.panes.get_mut(i) {
                    p.title = Some(t);
                }
                let _ = state.redraw();
            }
            Ok(PaneUpdate::Clear(i)) => {
                if let Some(p) = state.panes.get_mut(i) {
                    p.lines.clear();
                }
                let _ = state.redraw();
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Cheap polling redraw catches terminal resize between
                // pane updates. If size hasn't changed, redraw() is a
                // no-op-ish cost (a few hundred bytes of ANSI).
                if let Ok((w, h)) = terminal::size() {
                    if state.last_size != (w, h) {
                        let _ = state.redraw();
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[derive(Default, Clone)]
struct PaneState {
    title: Option<String>,
    lines: Vec<String>,
}

#[derive(Default)]
struct RendererState {
    kind: LayoutKind,
    percents: Vec<u8>,
    panes: Vec<PaneState>,
    last_size: (u16, u16),
}

impl RendererState {
    fn redraw(&mut self) -> io::Result<()> {
        let (w, h) = terminal::size()?;
        self.last_size = (w, h);
        if self.panes.is_empty() {
            return Ok(());
        }
        let regions = compute_regions(self.kind, &self.percents, w, h);

        let mut out = io::stdout().lock();
        queue!(out, terminal::Clear(terminal::ClearType::All))?;

        for (i, region) in regions.iter().enumerate() {
            let pane = &self.panes[i];

            // Title bar: " <title> " then horizontal rule to the edge.
            queue!(out, cursor::MoveTo(region.x, region.y))?;
            let title = pane.title.as_deref().unwrap_or("");
            let title_text = if title.is_empty() {
                String::new()
            } else {
                format!(" {title} ")
            };
            let pad_len = (region.w as usize).saturating_sub(title_text.chars().count());
            let rule = "─".repeat(pad_len);
            queue!(out, Print(format!("{title_text}{rule}")))?;

            // Content: last N lines that fit in the region (after the
            // 1-row title bar), truncated to region.w characters each.
            let content_h = region.h.saturating_sub(1) as usize;
            let start = pane.lines.len().saturating_sub(content_h);
            for (li, line) in pane.lines[start..].iter().enumerate() {
                queue!(
                    out,
                    cursor::MoveTo(region.x, region.y + 1 + li as u16)
                )?;
                let truncated: String = line.chars().take(region.w as usize).collect();
                queue!(out, Print(&truncated))?;
            }
        }

        out.flush()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Region {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

fn compute_regions(kind: LayoutKind, percents: &[u8], w: u16, h: u16) -> Vec<Region> {
    let total: u32 = percents.iter().map(|p| *p as u32).sum();
    if total == 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(percents.len());
    match kind {
        LayoutKind::Vertical => {
            let mut y = 0u16;
            for (i, p) in percents.iter().enumerate() {
                let height = if i + 1 == percents.len() {
                    h.saturating_sub(y)
                } else {
                    ((h as u32) * (*p as u32) / total) as u16
                };
                out.push(Region { x: 0, y, w, h: height });
                y = y.saturating_add(height);
            }
        }
        LayoutKind::Horizontal => {
            let mut x = 0u16;
            for (i, p) in percents.iter().enumerate() {
                let width = if i + 1 == percents.len() {
                    w.saturating_sub(x)
                } else {
                    ((w as u32) * (*p as u32) / total) as u16
                };
                out.push(Region { x, y: 0, w: width, h });
                x = x.saturating_add(width);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_split_sums_to_full_height() {
        let r = compute_regions(LayoutKind::Vertical, &[60, 40], 80, 24);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], Region { x: 0, y: 0, w: 80, h: 14 });
        // Last region absorbs rounding — 24 - 14 = 10.
        assert_eq!(r[1], Region { x: 0, y: 14, w: 80, h: 10 });
        assert_eq!(r[0].h + r[1].h, 24);
    }

    #[test]
    fn horizontal_split_three_panes() {
        let r = compute_regions(LayoutKind::Horizontal, &[33, 33, 34], 90, 30);
        assert_eq!(r.len(), 3);
        // Each pane spans full height.
        for region in &r {
            assert_eq!(region.h, 30);
            assert_eq!(region.y, 0);
        }
        // Widths sum to 90.
        let total_w: u16 = r.iter().map(|x| x.w).sum();
        assert_eq!(total_w, 90);
    }

    #[test]
    fn split_handles_uneven_percentages() {
        // Last pane absorbs rounding so the full screen is always
        // covered — `[20, 30, 50]` of 100 lines = exact division.
        let r = compute_regions(LayoutKind::Vertical, &[20, 30, 50], 80, 100);
        let total_h: u16 = r.iter().map(|x| x.h).sum();
        assert_eq!(total_h, 100);
    }

    #[test]
    fn zero_total_returns_empty() {
        let r = compute_regions(LayoutKind::Vertical, &[], 80, 24);
        assert!(r.is_empty());
    }
}
