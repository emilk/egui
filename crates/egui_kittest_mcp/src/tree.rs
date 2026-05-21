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
    match locator {
        Locator::Id { id } => find_by_id(&tree.state().root(), *id),
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
            let root = tree.state().root();
            first_match(&root, &filter)
        }
    }
}

fn find_by_id<'a>(node: &Node<'a>, target: u64) -> Option<Node<'a>> {
    if accesskit_id(node) == target {
        return Some(*node);
    }
    for child in node.children() {
        if let Some(found) = find_by_id(&child, target) {
            return Some(found);
        }
    }
    None
}

fn first_match<'a>(node: &Node<'a>, filter: &QueryFilter) -> Option<Node<'a>> {
    if matches(node, filter) {
        return Some(*node);
    }
    for child in node.children() {
        if let Some(found) = first_match(&child, filter) {
            return Some(found);
        }
    }
    None
}
