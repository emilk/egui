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
    /// Original `accesskit::NodeId` as a decimal string. Emitted as a string so the full
    /// u64 round-trips through MCP clients whose JSON parsers go through `f64` (which
    /// can't represent integers above 2^53 exactly).
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
    fn from_rect(r: accesskit::Rect) -> Self {
        Self {
            x: r.x0,
            y: r.y0,
            w: r.x1 - r.x0,
            h: r.y1 - r.y0,
        }
    }

    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct QueryFilter {
    /// AccessKit role name, e.g. `Button`, `Label`, `TextInput` (case-insensitive). An
    /// unrecognized role is rejected with an error that lists the roles present in the tree.
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

pub fn query(tree: &Tree, filter: &QueryFilter) -> Vec<NodeView> {
    let root = tree.state().root();
    let mut out = Vec::new();
    walk(&root, filter, &mut out);
    if out.len() > filter.limit {
        out.truncate(filter.limit);
    }
    out
}

fn walk(node: &Node<'_>, filter: &QueryFilter, out: &mut Vec<NodeView>) {
    if matches(node, filter) {
        out.push(node_view(node));
    }
    for child in node.children() {
        walk(&child, filter, out);
    }
}

/// Validate a `role` filter string against the full AccessKit role set.
///
/// Compared case-insensitively, the way [`matches`] compares. On failure the error lists the
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

pub fn node_view(node: &Node<'_>) -> NodeView {
    NodeView {
        id: accesskit_id(node).to_string(),
        role: format!("{:?}", node.role()),
        label: node.label(),
        value: node.value(),
        bounds: node.bounding_box().map(RectF::from_rect),
        focused: node.is_focused_in_tree(),
        disabled: node.is_disabled(),
        hidden: node.is_hidden(),
        parent_id: node.parent().map(|p| accesskit_id(&p).to_string()),
    }
}

/// Project a consumer node to its original `accesskit::NodeId` as a `u64`.
fn accesskit_id(node: &Node<'_>) -> u64 {
    let (local, _tree) = node.locate();
    local.0
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Locator {
    Id {
        /// Decimal string. Strings preserve the full u64 — JSON numbers above 2^53 lose
        /// precision in clients whose parsers go through `f64`, so we don't accept them.
        #[serde(deserialize_with = "deserialize_u64_from_string")]
        id: u64,
    },
    Match {
        #[serde(default)]
        role: Option<String>,
        #[serde(default)]
        label_contains: Option<String>,
    },
}

fn deserialize_u64_from_string<'de, D>(d: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    let s = String::deserialize(d)?;
    s.trim().parse::<u64>().map_err(D::Error::custom)
}

pub fn resolve_node<'a>(tree: &'a Tree, locator: &Locator) -> Option<Node<'a>> {
    let root = tree.state().root();
    match locator {
        Locator::Id { id } => find_first(&root, &|n| accesskit_id(n) == *id),
        Locator::Match {
            role,
            label_contains,
        } => {
            let filter = QueryFilter {
                role: role.clone(),
                label_contains: label_contains.clone(),
                visible_only: true,
                limit: 1,
            };
            find_first(&root, &|n| matches(n, &filter))
        }
    }
}

/// Depth-first search returning the first node satisfying `pred`.
fn find_first<'a>(node: &Node<'a>, pred: &impl Fn(&Node<'_>) -> bool) -> Option<Node<'a>> {
    if pred(node) {
        return Some(*node);
    }
    node.children().find_map(|child| find_first(&child, pred))
}
