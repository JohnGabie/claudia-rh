// Guarantees the browser viewport is at least MIN_W x MIN_H before the agent acts, so reading a page
// does not depend on the user's monitor or on how small the window got (spec 3.1).
//
// The window is the real one the user sees: it is resized with Browser.setWindowBounds, never faked
// with Emulation.setDeviceMetricsOverride. The decision is a pure function (`next_step`) so every
// case is a unit test; `ensure_min_viewport` only measures, applies the step and measures again.
use serde_json::{json, Value};
use std::time::Duration;

use super::cdp::CdpClient;

pub const MIN_W: u32 = 1200;
pub const MIN_H: u32 = 800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

impl WindowState {
    /// Chrome's `windowState` string. An unknown value counts as a normal window.
    pub fn parse(s: &str) -> Self {
        match s {
            "minimized" => Self::Minimized,
            "maximized" => Self::Maximized,
            "fullscreen" => Self::Fullscreen,
            _ => Self::Normal,
        }
    }
}

/// What the browser reports right now for the tab's window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measure {
    /// Page area in CSS pixels (what the page can lay out in).
    pub viewport: (u32, u32),
    /// Whole window, frame included.
    pub outer: (u32, u32),
    pub state: WindowState,
}

/// What was already tried in this call, so the planner never loops.
#[derive(Debug, Default, Clone, Copy)]
pub struct Tried {
    pub restore: bool,
    pub resize: bool,
    pub maximize: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Step {
    /// Big enough (or nothing can be measured): leave the window alone.
    Enough,
    /// Minimized: bring it back to a normal window first.
    Restore,
    /// Normal window too small: set this outer size (the frame was added on top of the minimum).
    Resize { outer: (u32, u32) },
    Maximize,
    /// Nothing more can be done (screen smaller than the minimum, or the browser refused).
    BelowMinimum,
}

/// Pure decision: the next thing to do for this measurement.
pub fn next_step(m: &Measure, tried: &Tried) -> Step {
    if m.state == WindowState::Minimized {
        return if tried.restore { Step::BelowMinimum } else { Step::Restore };
    }
    let (vw, vh) = m.viewport;
    if vw == 0 || vh == 0 {
        // A hidden tab reports no layout: there is nothing to size against.
        return Step::Enough;
    }
    if vw >= MIN_W && vh >= MIN_H {
        return Step::Enough;
    }
    if matches!(m.state, WindowState::Maximized | WindowState::Fullscreen) {
        return Step::BelowMinimum; // already as large as the screen allows
    }
    if !tried.resize {
        // Frame = what the window adds around the page (borders, tab strip, address bar, scrollbar).
        let frame_w = m.outer.0.saturating_sub(vw);
        let frame_h = m.outer.1.saturating_sub(vh);
        // Only grow: an axis that is already enough keeps its size.
        let w = if vw >= MIN_W { m.outer.0 } else { MIN_W + frame_w };
        let h = if vh >= MIN_H { m.outer.1 } else { MIN_H + frame_h };
        return Step::Resize { outer: (w, h) };
    }
    if !tried.maximize {
        return Step::Maximize;
    }
    Step::BelowMinimum
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportReport {
    pub before: (u32, u32),
    pub after: (u32, u32),
    pub adjusted: bool,
    pub below_minimum: bool,
}

impl ViewportReport {
    /// One line for the tool result, or None when there is nothing to say.
    pub fn note(&self) -> Option<String> {
        if self.below_minimum {
            Some(format!("viewport_abaixo_do_minimo: {}x{}", self.after.0, self.after.1))
        } else if self.adjusted {
            Some(format!(
                "viewport ajustado: {}x{} -> {}x{}",
                self.before.0, self.before.1, self.after.0, self.after.1
            ))
        } else {
            None
        }
    }
}

fn dim(v: &Value) -> u32 {
    v.as_f64().filter(|n| *n > 0.0).map(|n| n.round().min(u32::MAX as f64) as u32).unwrap_or(0)
}

/// Pure parsing of `Page.getLayoutMetrics` + `Browser.getWindowForTarget` results.
/// Returns the measurement and the window id.
pub fn parse_measure(layout: &Value, window: &Value) -> Result<(Measure, i64), String> {
    let vp = if layout["cssLayoutViewport"].is_object() { &layout["cssLayoutViewport"] } else { &layout["layoutViewport"] };
    let window_id = window["windowId"].as_i64().ok_or("Browser.getWindowForTarget returned no windowId")?;
    let b = &window["bounds"];
    Ok((
        Measure {
            viewport: (dim(&vp["clientWidth"]), dim(&vp["clientHeight"])),
            outer: (dim(&b["width"]), dim(&b["height"])),
            state: WindowState::parse(b["windowState"].as_str().unwrap_or("normal")),
        },
        window_id,
    ))
}

async fn measure(c: &CdpClient) -> Result<(Measure, i64), String> {
    let layout = c.call("Page.getLayoutMetrics", json!({})).await?;
    let window = c.call("Browser.getWindowForTarget", json!({})).await?;
    parse_measure(&layout, &window)
}

async fn set_bounds(c: &CdpClient, window_id: i64, bounds: Value) -> Result<(), String> {
    c.call("Browser.setWindowBounds", json!({ "windowId": window_id, "bounds": bounds })).await.map(|_| ())
}

/// The window takes a moment to lay the page out again after a resize, and the first reading that
/// differs can be an intermediate one (deciding on it would maximize a window that was just about to
/// be big enough). So wait until two consecutive readings agree, and the reading changed, or until
/// a patience limit passed with no change at all (the browser refused or clamped the request).
async fn measure_settled(c: &CdpClient, previous: &Measure, wanted_outer: Option<(u32, u32)>) -> Result<(Measure, i64), String> {
    let mut last = measure(c).await?;
    let mut stable = 0;
    for i in 0..20 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let now = measure(c).await?;
        if now.0 == last.0 {
            stable += 1;
        } else {
            stable = 0;
            last = now;
        }
        let missed = wanted_outer.is_some_and(|w| last.0.outer != w);
        if settle_done(stable, last.0 != *previous, i, missed) {
            break;
        }
    }
    Ok(last)
}

/// Polls with no change after which a request that nothing answered is taken as refused.
const PATIENCE_POLLS: u32 = 5;
/// Same, when we asked for a window size and the window does not have it yet: a loaded machine can
/// take much longer than half a second to apply `setWindowBounds` (CLA-36), and giving up too early
/// would maximize a window that was about to be big enough.
const PATIENCE_POLLS_SIZE_NOT_REACHED: u32 = 15;

/// Pure: may the settling loop stop? `stable` = consecutive equal readings, `changed` = the reading
/// differs from before the request, `poll` = how many polls passed (0-based), `missed` = a size was
/// requested and the window does not have it.
pub(crate) fn settle_done(stable: u32, changed: bool, poll: u32, missed: bool) -> bool {
    let patience = if missed { PATIENCE_POLLS_SIZE_NOT_REACHED } else { PATIENCE_POLLS };
    stable >= 2 && ((changed && !missed) || poll >= patience)
}

/// Makes sure the tab's window gives the page at least MIN_W x MIN_H. Never panics: a failure is
/// returned as Err and the caller ignores it (the agent's action must not fail because of this).
/// If the screen is too small to reach the minimum it does not fail either: `below_minimum` is set.
pub async fn ensure_min_viewport(c: &CdpClient) -> Result<ViewportReport, String> {
    ensure_min_viewport_with(c, None).await
}

/// True when a previous call already gave up at exactly this viewport: trying again would resize and
/// maximize the window on every tool call for nothing (a screen smaller than the minimum). If the user
/// changes the window the size differs and the attempt is made again.
pub fn should_skip(first_viewport: (u32, u32), gave_up_at: Option<(u32, u32)>) -> bool {
    gave_up_at == Some(first_viewport)
}

/// Like `ensure_min_viewport`, told where the last call gave up (`gave_up_at`): at that same size it does
/// not touch the window again and reports `below_minimum`.
///
/// Page zoom: the viewport is in CSS pixels and the window in device pixels, so at a zoom other than 100%
/// the frame computed for the resize is off. That is not corrected here: the planner converges anyway
/// (resize once, then maximize, then give up) and never loops.
pub async fn ensure_min_viewport_with(c: &CdpClient, gave_up_at: Option<(u32, u32)>) -> Result<ViewportReport, String> {
    let (first, window_id) = measure(c).await?;
    if should_skip(first.viewport, gave_up_at) {
        return Ok(ViewportReport { before: first.viewport, after: first.viewport, adjusted: false, below_minimum: true });
    }
    let mut current = first.clone();
    let mut tried = Tried::default();
    let mut below_minimum = false;
    // At most restore + resize + maximize, each followed by a measurement.
    for _ in 0..4 {
        // The outer size the window should have once this step is applied (only a resize names one).
        let mut wanted_outer = None;
        match next_step(&current, &tried) {
            Step::Enough => break,
            Step::BelowMinimum => {
                below_minimum = true;
                break;
            }
            Step::Restore => {
                tried.restore = true;
                set_bounds(c, window_id, json!({ "windowState": "normal" })).await?;
            }
            Step::Resize { outer } => {
                tried.resize = true;
                wanted_outer = Some(outer);
                set_bounds(c, window_id, json!({ "windowState": "normal", "width": outer.0, "height": outer.1 })).await?;
            }
            Step::Maximize => {
                tried.maximize = true;
                set_bounds(c, window_id, json!({ "windowState": "maximized" })).await?;
            }
        }
        current = measure_settled(c, &current, wanted_outer).await?.0;
    }
    let after = current.viewport;
    let known = after.0 > 0 && after.1 > 0;
    Ok(ViewportReport {
        before: first.viewport,
        after,
        adjusted: after != first.viewport,
        below_minimum: below_minimum && known && (after.0 < MIN_W || after.1 < MIN_H),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(viewport: (u32, u32), outer: (u32, u32), state: WindowState) -> Measure {
        Measure { viewport, outer, state }
    }

    #[test]
    fn a_window_where_a_previous_call_gave_up_is_not_touched_again_until_it_changes() {
        assert!(should_skip((1100, 700), Some((1100, 700))), "same size as where we gave up: leave it");
        assert!(!should_skip((1100, 700), None), "never gave up: try");
        assert!(!should_skip((900, 600), Some((1100, 700))), "the user changed the window: try again");
    }

    #[test]
    fn a_big_enough_window_is_left_alone() {
        let big = m((1920, 889), (1936, 1040), WindowState::Normal);
        assert_eq!(next_step(&big, &Tried::default()), Step::Enough);
        let exact = m((MIN_W, MIN_H), (1216, 951), WindowState::Normal);
        assert_eq!(next_step(&exact, &Tried::default()), Step::Enough, "the minimum itself is enough");
    }

    #[test]
    fn a_small_normal_window_is_resized_by_the_minimum_plus_its_frame() {
        // The user's real window: 945x1012 outside, 929x861 viewport => frame 16x151.
        let small = m((929, 861), (945, 1012), WindowState::Normal);
        assert_eq!(
            next_step(&small, &Tried::default()),
            Step::Resize { outer: (MIN_W + 16, 1012) },
            "width grows to the minimum + frame; the height (861 >= 800) is kept"
        );
    }

    #[test]
    fn a_big_frame_is_added_on_top_of_the_minimum() {
        let small = m((700, 300), (900, 560), WindowState::Normal);
        assert_eq!(next_step(&small, &Tried::default()), Step::Resize { outer: (MIN_W + 200, MIN_H + 260) });
    }

    #[test]
    fn an_axis_that_is_already_enough_is_never_shrunk() {
        let wide_and_short = m((1600, 500), (1616, 651), WindowState::Normal);
        assert_eq!(next_step(&wide_and_short, &Tried::default()), Step::Resize { outer: (1616, MIN_H + 151) });
    }

    #[test]
    fn a_minimized_window_is_restored_first_and_only_once() {
        let min = m((0, 0), (160, 28), WindowState::Minimized);
        assert_eq!(next_step(&min, &Tried::default()), Step::Restore);
        assert_eq!(next_step(&min, &Tried { restore: true, ..Tried::default() }), Step::BelowMinimum, "never loops on a window that will not restore");
    }

    #[test]
    fn a_maximized_window_that_is_big_enough_is_left_alone() {
        let maxed = m((1920, 940), (1936, 1040), WindowState::Maximized);
        assert_eq!(next_step(&maxed, &Tried::default()), Step::Enough);
    }

    #[test]
    fn a_maximized_window_on_a_small_screen_gives_up_without_failing() {
        let tiny_screen = m((1100, 700), (1116, 851), WindowState::Maximized);
        assert_eq!(next_step(&tiny_screen, &Tried::default()), Step::BelowMinimum, "maximized is as big as it gets");
        let fullscreen = m((1024, 768), (1024, 768), WindowState::Fullscreen);
        assert_eq!(next_step(&fullscreen, &Tried::default()), Step::BelowMinimum);
    }

    #[test]
    fn if_the_resize_is_not_enough_it_maximizes_then_gives_up() {
        // The screen clamped the resize: still too small and still a normal window.
        let clamped = m((1100, 700), (1116, 851), WindowState::Normal);
        let t1 = Tried { resize: true, ..Tried::default() };
        assert_eq!(next_step(&clamped, &t1), Step::Maximize);
        let t2 = Tried { resize: true, maximize: true, ..Tried::default() };
        assert_eq!(next_step(&clamped, &t2), Step::BelowMinimum);
    }

    #[test]
    fn a_requested_size_that_the_window_does_not_have_yet_is_waited_for_longer() {
        // CLA-36: on a loaded machine the window took longer than half a second to take the new size,
        // and the loop gave up and maximized a window that was about to be big enough.
        assert!(!settle_done(2, false, 5, true), "asked for a size, not there after 0.6 s: keep waiting");
        assert!(!settle_done(2, true, 5, true), "an intermediate reading that is not the size asked for is not final");
        assert!(settle_done(2, false, 15, true), "but not forever: a clamped request ends");
        assert!(settle_done(2, true, 3, false), "the size was reached (or none was asked for) and the reading changed");
        assert!(settle_done(2, false, 5, false), "nothing asked and nothing changed for half a second: refused");
        assert!(!settle_done(1, true, 9, false), "two equal readings are always required");
    }

    #[test]
    fn a_tab_with_no_layout_is_left_alone() {
        let hidden = m((0, 0), (945, 1012), WindowState::Normal);
        assert_eq!(next_step(&hidden, &Tried::default()), Step::Enough, "no viewport to measure, nothing to fix");
    }

    #[test]
    fn the_note_is_one_line_or_nothing() {
        let ok = ViewportReport { before: (1920, 889), after: (1920, 889), adjusted: false, below_minimum: false };
        assert_eq!(ok.note(), None);
        let fixed = ViewportReport { before: (929, 861), after: (1200, 861), adjusted: true, below_minimum: false };
        assert_eq!(fixed.note().as_deref(), Some("viewport ajustado: 929x861 -> 1200x861"));
        let small = ViewportReport { before: (1100, 700), after: (1100, 700), adjusted: false, below_minimum: true };
        assert_eq!(small.note().as_deref(), Some("viewport_abaixo_do_minimo: 1100x700"));
        let both = ViewportReport { before: (700, 400), after: (1100, 700), adjusted: true, below_minimum: true };
        assert_eq!(both.note().as_deref(), Some("viewport_abaixo_do_minimo: 1100x700"), "a screen that is too small is the news");
    }

    #[test]
    fn the_browser_answers_are_parsed_and_garbage_does_not_panic() {
        let layout = json!({ "cssLayoutViewport": { "clientWidth": 929, "clientHeight": 861.4 } });
        let window = json!({ "windowId": 7, "bounds": { "left": 10, "top": 20, "width": 945, "height": 1012, "windowState": "normal" } });
        let (meas, id) = parse_measure(&layout, &window).unwrap();
        assert_eq!(id, 7);
        assert_eq!(meas, m((929, 861), (945, 1012), WindowState::Normal));

        let max = json!({ "windowId": 1, "bounds": { "width": 1936, "height": 1040, "windowState": "maximized" } });
        assert_eq!(parse_measure(&layout, &max).unwrap().0.state, WindowState::Maximized);

        assert!(parse_measure(&layout, &json!({})).is_err(), "no windowId is an error, not a panic");
        let (zero, _) = parse_measure(&json!({}), &window).unwrap();
        assert_eq!(zero.viewport, (0, 0), "missing metrics read as 'no layout'");
        let (odd, _) = parse_measure(&json!({ "layoutViewport": { "clientWidth": -5, "clientHeight": "x" } }), &window).unwrap();
        assert_eq!(odd.viewport, (0, 0));
        assert_eq!(WindowState::parse("something-new"), WindowState::Normal);
    }
}
