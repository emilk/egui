//! Helpers that flatten the accesskit tree into MCP-friendly shapes.
//!
//! Note: `accesskit_consumer::NodeId` is a private composite (tree-index + local-id) and
//! can't be constructed from outside the crate. We project everything externally as the
//! original `accesskit::NodeId` (a `pub u64`), and look up by walking the tree.

use accesskit_consumer::{Node, Tree};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NodeView {
    /// Node id, used with `click`, `type_text`, and `get_node`.
    pub id: String,
    pub role: String,
    pub label: Option<String>,
    pub value: Option<String>,
    pub bounds: Option<RectF>,
    pub focused: bool,
    pub disabled: bool,
    pub hidden: bool,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
pub struct RectF {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl RectF {
    /// Build from AccessKit physical-pixel bounds, scaling to logical points (the consumer's
    /// `bounding_box` applies egui's root `scale(pixels_per_point)` transform, so bounds arrive
    /// in physical pixels — divide them back out).
    fn from_physical(r: accesskit::Rect, pixels_per_point: f32) -> Self {
        let s = 1.0 / f64::from(pixels_per_point);
        Self {
            x: r.x0 * s,
            y: r.y0 * s,
            w: (r.x1 - r.x0) * s,
            h: (r.y1 - r.y0) * s,
        }
    }

    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct QueryFilter {
    /// Role name, e.g. `Button`, `Label`, `TextInput` (case-insensitive).
    /// An unrecognized role is rejected with an error that lists the roles present in the tree.
    pub role: Option<String>,
    pub label_contains: Option<String>,
    #[serde(default = "default_true")]
    pub visible_only: bool,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_true() -> bool {
    true
}

fn default_limit() -> usize {
    200
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self {
            role: None,
            label_contains: None,
            visible_only: true,
            limit: default_limit(),
        }
    }
}

pub fn query(tree: &Tree, filter: &QueryFilter, pixels_per_point: f32) -> Vec<NodeView> {
    let root = tree.state().root();
    let mut out = Vec::new();
    walk(&root, filter, pixels_per_point, &mut out);
    if out.len() > filter.limit {
        out.truncate(filter.limit);
    }
    out
}

fn walk(node: &Node<'_>, filter: &QueryFilter, pixels_per_point: f32, out: &mut Vec<NodeView>) {
    if matches(node, filter) {
        out.push(node_view(node, pixels_per_point));
    }
    for child in node.children() {
        walk(&child, filter, pixels_per_point, out);
    }
}

/// Validate a `role` filter string against the full AccessKit role set.
///
/// Compared case-insensitively, the way `matches` compares. On failure the error lists the
/// distinct roles actually present in `tree`, so the agent learns what it can filter by instead of
/// getting a silent empty result. Validity is checked against *all* roles, not just those present,
/// so polling tools like `wait_for` can still wait for a valid role that hasn't appeared yet.
///
/// # Errors
/// If `role` is not a known AccessKit role name.
pub fn validate_role(role: &str, tree: Option<&Tree>) -> Result<(), String> {
    // `accesskit::Role` is `#[repr(u8)]` with `enumn::N`, so walking `n(0), n(1), …` until `None`
    // enumerates every variant; `{:?}` yields the same name `matches`/`node_view` expose.
    let valid = (0u8..=u8::MAX)
        .map_while(accesskit::Role::n)
        .any(|r| role.eq_ignore_ascii_case(&format!("{r:?}")));
    if valid {
        return Ok(());
    }
    let present = tree.map(roles_in_tree).unwrap_or_default();
    let hint = if present.is_empty() {
        "(no nodes in the current tree)".to_owned()
    } else {
        present.join(", ")
    };
    Err(format!(
        "unknown role `{role}` — roles present in the current tree: {hint}"
    ))
}

/// The distinct AccessKit roles present anywhere in `tree`, sorted, as their display names.
fn roles_in_tree(tree: &Tree) -> Vec<String> {
    let mut roles = std::collections::BTreeSet::new();
    collect_roles(&tree.state().root(), &mut roles);
    roles.into_iter().collect()
}

fn collect_roles(node: &Node<'_>, out: &mut std::collections::BTreeSet<String>) {
    out.insert(format!("{:?}", node.role()));
    for child in node.children() {
        collect_roles(&child, out);
    }
}

fn matches(node: &Node<'_>, filter: &QueryFilter) -> bool {
    if filter.visible_only && node.is_hidden() {
        return false;
    }
    if let Some(role) = &filter.role
        && !role.eq_ignore_ascii_case(&format!("{:?}", node.role()))
    {
        return false;
    }
    if let Some(needle) = &filter.label_contains {
        let hay = node.label().unwrap_or_default();
        if !hay
            .to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase())
        {
            return false;
        }
    }
    true
}

pub fn node_view(node: &Node<'_>, pixels_per_point: f32) -> NodeView {
    NodeView {
        id: accesskit_id(node).to_string(),
        role: format!("{:?}", node.role()),
        label: node.label(),
        value: node.value(),
        bounds: node
            .bounding_box()
            .map(|r| RectF::from_physical(r, pixels_per_point)),
        focused: node.is_focused_in_tree(),
        disabled: node.is_disabled(),
        hidden: node.is_hidden(),
        parent_id: node.parent().map(|p| accesskit_id(&p).to_string()),
    }
}

/// Project a consumer node to its original `accesskit::NodeId` as a `u64`.
pub fn accesskit_id(node: &Node<'_>) -> u64 {
    let (local, _tree) = node.locate();
    local.0
}

/// A resolved lookup target: a specific node `id`, or a role/label match.
/// Built directly by the tools from a `Target` — never deserialized.
#[derive(Debug, Clone)]
pub enum Locator {
    Id {
        id: u64,
    },
    Match {
        role: Option<String>,
        label_contains: Option<String>,
    },
}

impl Locator {
    /// Build a locator from raw tool fields: a parseable `id` wins, else a role/label match.
    /// Returns `None` when no locator field is set.
    pub fn from_fields(
        id: Option<&str>,
        role: Option<String>,
        label_contains: Option<String>,
    ) -> Option<Self> {
        if let Some(id) = id.and_then(|s| s.trim().parse::<u64>().ok()) {
            return Some(Self::Id { id });
        }
        if role.is_some() || label_contains.is_some() {
            return Some(Self::Match {
                role,
                label_contains,
            });
        }
        None
    }
}

/// Resolve a locator to *exactly one* node for an action (click, focus, …).
///
/// Like kittest's `get_by_*`, this is strict: an ambiguous locator is an error, not a silent
/// "first match wins". A specific `id` resolves at most one node; a `role`/`label_contains` match
/// errors if it hits zero or more than one node, listing the candidates so the caller can narrow
/// the filter or target a specific `id`. Use `query_tree` (which returns all matches) when you
/// genuinely expect several.
///
/// # Errors
/// If no node matches, or if more than one does.
pub fn resolve_unique<'a>(
    tree: &'a Tree,
    locator: &Locator,
    pixels_per_point: f32,
) -> Result<Node<'a>, String> {
    let root = tree.state().root();
    match locator {
        Locator::Id { id } => {
            let mut found = Vec::new();
            find_all(&root, &|n| accesskit_id(n) == *id, &mut found);
            one(found, pixels_per_point, &format!("id `{id}`"))
        }
        Locator::Match {
            role,
            label_contains,
        } => {
            let filter = QueryFilter {
                role: role.clone(),
                label_contains: label_contains.clone(),
                visible_only: true,
                limit: usize::MAX,
            };
            let mut found = Vec::new();
            find_all(&root, &|n| matches(n, &filter), &mut found);
            let what = describe_match(role.as_deref(), label_contains.as_deref());
            one(found, pixels_per_point, &what)
        }
    }
}

/// Reduce a match list to the single node an action needs, or an error describing the miss.
fn one<'a>(
    mut found: Vec<Node<'a>>,
    pixels_per_point: f32,
    what: &str,
) -> Result<Node<'a>, String> {
    match found.len() {
        0 => Err(format!("no node found matching {what}")),
        1 => Ok(found.remove(0)),
        n => {
            let views: Vec<NodeView> = found
                .iter()
                .map(|node| node_view(node, pixels_per_point))
                .collect();
            let list =
                serde_json::to_string_pretty(&views).unwrap_or_else(|_| format!("{n} nodes"));
            Err(format!(
                "{what} matched {n} nodes — narrow `role`/`label_contains`, or target a specific `id`. Matches:\n{list}"
            ))
        }
    }
}

fn describe_match(role: Option<&str>, label_contains: Option<&str>) -> String {
    match (role, label_contains) {
        (Some(r), Some(l)) => format!("role `{r}` with label containing `{l}`"),
        (Some(r), None) => format!("role `{r}`"),
        (None, Some(l)) => format!("label containing `{l}`"),
        (None, None) => "the locator".to_owned(),
    }
}

/// Depth-first collection of every node satisfying `pred`.
fn find_all<'a>(node: &Node<'a>, pred: &impl Fn(&Node<'_>) -> bool, out: &mut Vec<Node<'a>>) {
    if pred(node) {
        out.push(*node);
    }
    for child in node.children() {
        find_all(&child, pred, out);
    }
}
