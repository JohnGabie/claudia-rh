// A hint, never a block: when the same tool is called again and again on the same target while the page
// does not change (URL and the content last read both the same), the result carries a line telling the
// model to read differently or give up. The session 150 log showed the shape: `Tab` x5, then a
// `browser_crop`, over and over on a page whose text the model could not read.
//
// The window looks at the last few calls (not only the consecutive ones), so an alternation like
// Tab, Tab, crop, Tab, Tab, crop still adds up. A call counts only when the page signature is the same
// as the new call's: when the page changes the old calls stop counting, which is the reset.
use std::collections::VecDeque;

/// How many recent calls are remembered.
const WINDOW: usize = 8;
/// Equal calls (tool + target + page signature) in the window that trigger the hint.
pub const THRESHOLD: usize = 4;

struct Call {
    tool: String,
    target: String,
    signature: String,
}

#[derive(Default)]
pub struct LoopGuard {
    recent: VecDeque<Call>,
}

impl LoopGuard {
    /// Records a call and returns the hint when it is the `THRESHOLD`-th (or later) equal call.
    /// It never refuses anything: the caller appends the text to the result.
    pub fn observe(&mut self, tool: &str, target: &str, signature: &str) -> Option<String> {
        self.recent.push_back(Call { tool: tool.to_string(), target: target.to_string(), signature: signature.to_string() });
        while self.recent.len() > WINDOW {
            self.recent.pop_front();
        }
        let same = self.recent.iter().filter(|c| c.tool == tool && c.target == target && c.signature == signature).count();
        (same >= THRESHOLD).then(|| {
            format!(
                "Dica: você repetiu {tool} {same} vezes sem a página mudar. Use browser_text para ler, browser_scroll para ver mais, ou marque a vaga como pendente."
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fourth_equal_call_gets_the_hint_and_the_first_three_do_not() {
        let mut g = LoopGuard::default();
        for i in 1..=3 {
            assert_eq!(g.observe("browser_key", "Tab", "pagina-1"), None, "call {i}");
        }
        let hint = g.observe("browser_key", "Tab", "pagina-1").expect("the 4th repeat");
        assert!(hint.contains("browser_key") && hint.contains("4 vezes") && hint.contains("browser_text") && hint.contains("browser_scroll"), "{hint}");
        assert!(g.observe("browser_key", "Tab", "pagina-1").unwrap().contains("5 vezes"), "it keeps counting");
    }

    #[test]
    fn it_never_blocks_it_only_returns_text() {
        // The API has no way to refuse: a legitimate repeated click still goes through, with a hint.
        let mut g = LoopGuard::default();
        let mut hints = 0;
        for _ in 0..20 {
            if g.observe("browser_click", "Próximo", "s").is_some() {
                hints += 1;
            }
        }
        assert_eq!(hints, 17, "calls 4..=20 carry the hint, none of them is refused");
    }

    #[test]
    fn a_different_target_or_tool_is_not_a_repeat() {
        let mut g = LoopGuard::default();
        for t in ["Tab", "Enter", "Escape", "Space", "Tab", "Enter"] {
            assert_eq!(g.observe("browser_key", t, "s"), None, "{t}");
        }
        for tool in ["browser_read", "browser_text", "browser_scroll", "browser_crop"] {
            assert_eq!(g.observe(tool, "Tab", "s"), None, "{tool}");
        }
    }

    #[test]
    fn it_resets_when_the_page_changes() {
        let mut g = LoopGuard::default();
        for _ in 0..3 {
            assert_eq!(g.observe("browser_key", "Tab", "pagina-1"), None);
        }
        // The page changed (new URL or new content): the earlier calls no longer count.
        assert_eq!(g.observe("browser_key", "Tab", "pagina-2"), None);
        assert_eq!(g.observe("browser_key", "Tab", "pagina-2"), None);
        assert_eq!(g.observe("browser_key", "Tab", "pagina-2"), None);
        assert!(g.observe("browser_key", "Tab", "pagina-2").is_some(), "four on the new page: the hint comes back");
    }

    #[test]
    fn an_alternation_with_another_call_still_adds_up() {
        // The shape of the session 150 log: Tab, Tab, crop, Tab, Tab, crop, ...
        let mut g = LoopGuard::default();
        let mut hinted_on = None;
        for round in 0..3 {
            for (i, (tool, target)) in [("browser_key", "Tab"), ("browser_key", "Tab"), ("browser_crop", "10,20,300,200")].into_iter().enumerate() {
                if g.observe(tool, target, "s").is_some() && hinted_on.is_none() {
                    hinted_on = Some((round, i));
                }
            }
        }
        assert_eq!(hinted_on, Some((1, 1)), "the 4th Tab, in the second round");
    }

    #[test]
    fn old_calls_fall_out_of_the_window() {
        let mut g = LoopGuard::default();
        for _ in 0..3 {
            g.observe("browser_key", "Tab", "s");
        }
        // Eight different calls push the three Tabs out of memory.
        for i in 0..WINDOW {
            g.observe("browser_click", &format!("botão {i}"), "s");
        }
        assert_eq!(g.observe("browser_key", "Tab", "s"), None, "only one Tab is left in the window");
    }
}
