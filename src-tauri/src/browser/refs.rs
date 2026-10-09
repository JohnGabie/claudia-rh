// Turns Accessibility.getFullAXTree into short refs (e1, e2, ...) the agent can
// click by. The accessibility tree is where the accessible name lives; it is
// what made "12 of 12 named fields" possible in the prototype.
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const INTERACTIVE: &[&str] = &[
    "button", "link", "textbox", "searchbox", "combobox", "radio", "checkbox",
    "menuitem", "tab", "option", "switch", "spinbutton", "listbox",
];

#[derive(Debug, Clone, PartialEq)]
pub struct RefNode {
    pub reference: String,
    pub role: String,
    pub name: String,
    pub backend_node_id: i64,
    pub disabled: bool,
}

fn val(v: &Value) -> String {
    v["value"].as_str().unwrap_or("").to_string()
}

fn is_disabled(n: &Value) -> bool {
    n["properties"].as_array().is_some_and(|ps| {
        ps.iter().any(|p| p["name"] == "disabled" && p["value"]["value"] == true)
    })
}

pub fn build_refs(ax: &Value, scope: Option<&str>) -> Result<Vec<RefNode>, String> {
    let nodes = ax["nodes"].as_array().cloned().unwrap_or_default();
    let by_id: HashMap<String, &Value> = nodes
        .iter()
        .filter_map(|n| n["nodeId"].as_str().map(|id| (id.to_string(), n)))
        .collect();

    let allowed: Option<HashSet<String>> = match scope {
        None => None,
        Some(s) => {
            let needle = s.to_lowercase();
            let root = nodes.iter().find(|n| {
                val(&n["role"]).to_lowercase() == needle || val(&n["name"]).to_lowercase().contains(&needle)
            });
            let Some(root) = root else {
                return Err(format!("scope \"{s}\" not found in the page"));
            };
            let mut seen = HashSet::new();
            let mut stack = vec![root["nodeId"].as_str().unwrap_or("").to_string()];
            while let Some(id) = stack.pop() {
                if !seen.insert(id.clone()) {
                    continue;
                }
                if let Some(n) = by_id.get(&id) {
                    for c in n["childIds"].as_array().into_iter().flatten() {
                        if let Some(cid) = c.as_str() {
                            stack.push(cid.to_string());
                        }
                    }
                }
            }
            Some(seen)
        }
    };

    let mut out = Vec::new();
    for n in &nodes {
        let id = n["nodeId"].as_str().unwrap_or("");
        if let Some(a) = &allowed {
            if !a.contains(id) {
                continue;
            }
        }
        if n["ignored"] == true {
            continue;
        }
        let role = val(&n["role"]);
        if !INTERACTIVE.contains(&role.as_str()) {
            continue;
        }
        let name = val(&n["name"]);
        if name.is_empty() && role != "textbox" && role != "searchbox" {
            continue;
        }
        let Some(backend) = n["backendDOMNodeId"].as_i64() else { continue };
        out.push(RefNode {
            reference: format!("e{}", out.len() + 1),
            role,
            name,
            backend_node_id: backend,
            disabled: is_disabled(n),
        });
    }
    Ok(out)
}

pub fn render(nodes: &[RefNode]) -> String {
    if nodes.is_empty() {
        return "(nenhum elemento interativo encontrado)".to_string();
    }
    nodes
        .iter()
        .map(|n| {
            let name = if n.name.is_empty() { "(sem nome)" } else { &n.name };
            format!("{} {} \"{}\"{}", n.reference, n.role, name, if n.disabled { " (disabled)" } else { "" })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn find<'a>(nodes: &'a [RefNode], reference: &str) -> Option<&'a RefNode> {
    nodes.iter().find(|n| n.reference == reference)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tree() -> Value {
        json!({ "nodes": [
            { "nodeId": "1", "ignored": false, "role": {"value": "RootWebArea"}, "name": {"value": "fixture"}, "childIds": ["2", "6", "7"], "backendDOMNodeId": 1 },
            { "nodeId": "2", "ignored": false, "role": {"value": "dialog"}, "name": {"value": "Candidatar-se"}, "childIds": ["3", "4", "5"], "backendDOMNodeId": 2 },
            { "nodeId": "3", "ignored": false, "role": {"value": "textbox"}, "name": {"value": "Nome"}, "backendDOMNodeId": 3 },
            { "nodeId": "4", "ignored": false, "role": {"value": "button"}, "name": {"value": "Avançar"}, "backendDOMNodeId": 4 },
            { "nodeId": "5", "ignored": true,  "role": {"value": "button"}, "name": {"value": "Escondido"}, "backendDOMNodeId": 5 },
            { "nodeId": "6", "ignored": false, "role": {"value": "link"},   "name": {"value": "Fora do modal"}, "backendDOMNodeId": 6 },
            { "nodeId": "7", "ignored": false, "role": {"value": "button"}, "name": {"value": "Desligado"},
              "properties": [{"name": "disabled", "value": {"type": "boolean", "value": true}}], "backendDOMNodeId": 7 }
        ]})
    }

    #[test]
    fn keeps_only_visible_interactive_nodes_with_sequential_refs() {
        let n = build_refs(&tree(), None).unwrap();
        let got: Vec<(&str, &str, &str)> = n.iter().map(|r| (r.reference.as_str(), r.role.as_str(), r.name.as_str())).collect();
        assert_eq!(got, vec![
            ("e1", "textbox", "Nome"), ("e2", "button", "Avançar"),
            ("e3", "link", "Fora do modal"), ("e4", "button", "Desligado"),
        ]);
        assert!(n[3].disabled);
        assert_eq!(n[1].backend_node_id, 4);
    }

    #[test]
    fn scope_limits_to_the_descendants_of_the_matching_node() {
        let n = build_refs(&tree(), Some("dialog")).unwrap();
        let names: Vec<&str> = n.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["Nome", "Avançar"]);
    }

    #[test]
    fn scope_matches_by_name_too_and_ignores_case() {
        let n = build_refs(&tree(), Some("candidatar")).unwrap();
        assert_eq!(n.len(), 2);
    }

    #[test]
    fn unknown_scope_is_an_error_not_an_empty_list() {
        let e = build_refs(&tree(), Some("nada disto")).unwrap_err();
        assert!(e.contains("scope"), "{e}");
    }

    #[test]
    fn render_and_find() {
        let n = build_refs(&tree(), None).unwrap();
        let text = render(&n);
        assert!(text.contains("e2 button \"Avançar\""), "{text}");
        assert!(text.contains("e4 button \"Desligado\" (disabled)"), "{text}");
        assert_eq!(find(&n, "e3").unwrap().name, "Fora do modal");
        assert!(find(&n, "e99").is_none());
    }

    #[test]
    fn empty_page_renders_a_hint() {
        let n = build_refs(&json!({ "nodes": [] }), None).unwrap();
        assert!(render(&n).contains("nenhum"));
    }

    #[test]
    fn skips_interactive_elements_with_empty_names_except_textbox_searchbox() {
        let tree = json!({ "nodes": [
            { "nodeId": "1", "ignored": false, "role": {"value": "button"}, "name": {"value": ""}, "backendDOMNodeId": 1 },
            { "nodeId": "2", "ignored": false, "role": {"value": "textbox"}, "name": {"value": ""}, "backendDOMNodeId": 2 },
            { "nodeId": "3", "ignored": false, "role": {"value": "searchbox"}, "name": {"value": ""}, "backendDOMNodeId": 3 }
        ]});
        let n = build_refs(&tree, None).unwrap();
        let refs: Vec<&str> = n.iter().map(|r| r.reference.as_str()).collect();
        // Only textbox and searchbox should be included; empty-named button should be excluded
        assert_eq!(refs, vec!["e1", "e2"]);
    }
}

/// Hands out refs that are STABLE per element inside one document: the same node always gets the
/// same ref, a number is never reused, and a narrower (scoped) read never renumbers anything.
/// Without this, `browser_read` with a scope would silently re-point every old ref at a different
/// element, and a click on "e3" could land on the wrong button.
#[derive(Default)]
pub struct RefBook {
    doc: String,
    next: u64,
    by_backend: std::collections::HashMap<i64, String>,
    nodes: std::collections::HashMap<String, RefNode>,
}

impl RefBook {
    /// Registers a read of the page and returns its nodes carrying the book's stable refs.
    /// A different doc (the page changed) starts a fresh book; the same document keeps every ref
    /// already handed out, so a scoped read cannot renumber the full one.
    pub fn update(&mut self, doc: &str, mut fresh: Vec<RefNode>) -> Vec<RefNode> {
        if self.doc != doc {
            *self = RefBook { doc: doc.to_string(), ..Default::default() };
        }
        let RefBook { next, by_backend, nodes, .. } = self;
        for n in fresh.iter_mut() {
            let reference = by_backend
                .entry(n.backend_node_id)
                .or_insert_with(|| {
                    *next += 1;
                    format!("e{next}")
                })
                .clone();
            n.reference = reference.clone();
            nodes.insert(reference, n.clone());
        }
        fresh
    }

    /// The element a ref points to, only while the page is still the document it was read in.
    pub fn find(&self, doc: &str, reference: &str) -> Option<&RefNode> {
        if self.doc != doc {
            return None;
        }
        self.nodes.get(reference)
    }

    /// True when refs exist for exactly this document.
    pub fn is_current(&self, doc: &str) -> bool {
        !self.doc.is_empty() && self.doc == doc
    }
}

#[cfg(test)]
mod refbook_tests {
    use super::*;

    fn node(backend: i64, name: &str) -> RefNode {
        // The reference a fresh `build_refs` would give is meaningless here: the book assigns its own.
        RefNode { reference: "e1".into(), role: "button".into(), name: name.into(), backend_node_id: backend, disabled: false }
    }

    #[test]
    fn the_same_element_keeps_its_ref_across_reads() {
        let mut book = RefBook::default();
        let full = book.update("doc1", vec![node(10, "A"), node(11, "B"), node(12, "C")]);
        let b_ref = full[1].reference.clone();
        // A SCOPED read returns only B, renumbered from e1 by build_refs: B must keep its old ref.
        let scoped = book.update("doc1", vec![node(11, "B")]);
        assert_eq!(scoped[0].reference, b_ref, "a narrower read must not renumber the element");
        assert_eq!(book.find("doc1", &b_ref).unwrap().backend_node_id, 11);
    }

    #[test]
    fn a_new_element_never_reuses_a_ref_that_was_handed_out() {
        let mut book = RefBook::default();
        let first = book.update("doc1", vec![node(10, "A"), node(11, "B")]);
        let second = book.update("doc1", vec![node(11, "B"), node(20, "New")]);
        let new_ref = &second[1].reference;
        assert!(first.iter().all(|n| &n.reference != new_ref), "{new_ref} was already given to another element");
    }

    #[test]
    fn refs_of_elements_missing_from_a_later_read_stay_resolvable() {
        let mut book = RefBook::default();
        let full = book.update("doc1", vec![node(10, "A"), node(11, "B")]);
        let a_ref = full[0].reference.clone();
        book.update("doc1", vec![node(11, "B")]); // a scoped read that does not include A
        assert_eq!(book.find("doc1", &a_ref).unwrap().name, "A", "an earlier ref still points to ITS element");
    }

    #[test]
    fn a_new_document_starts_a_new_book() {
        let mut book = RefBook::default();
        let old = book.update("doc1", vec![node(10, "A")]);
        let old_ref = old[0].reference.clone();
        // A DIFFERENT backend id in the new page: if the old book leaked, it would get e2, not e1.
        let fresh = book.update("doc2", vec![node(99, "Other page")]);
        assert!(book.find("doc1", &old_ref).is_none(), "refs of the previous document are gone");
        assert_eq!(fresh[0].reference, "e1", "numbering restarts with the document");
        assert_eq!(book.find("doc2", "e1").unwrap().name, "Other page", "e1 of the new page is the new element, not the old one");
    }

    #[test]
    fn a_ref_is_only_valid_for_the_document_it_was_read_in() {
        let mut book = RefBook::default();
        let r = book.update("doc1", vec![node(10, "A")])[0].reference.clone();
        assert!(book.find("doc1", &r).is_some());
        assert!(book.find("doc2", &r).is_none(), "the page changed: the ref must not resolve");
        assert!(!book.is_current("doc2"));
        assert!(book.is_current("doc1"));
        assert!(!RefBook::default().is_current(""), "an empty book is never current");
    }

    #[test]
    fn the_latest_read_wins_for_the_data_of_an_element() {
        let mut book = RefBook::default();
        let r = book.update("doc1", vec![node(10, "Avançar")])[0].reference.clone();
        book.update("doc1", vec![node(10, "Enviar candidatura")]);
        assert_eq!(book.find("doc1", &r).unwrap().name, "Enviar candidatura");
    }

    #[test]
    fn an_unknown_ref_resolves_to_nothing() {
        let mut book = RefBook::default();
        book.update("doc1", vec![node(10, "A")]);
        assert!(book.find("doc1", "e999").is_none());
    }
}
