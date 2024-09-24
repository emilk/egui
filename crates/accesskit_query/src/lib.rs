use accesskit_consumer::{FilterResult, Node};
use extension_traits::extension;
use std::fmt::Debug;

struct DebugNode<'a>(&'a Node<'a>);

impl Debug for DebugNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let node = self.0;

        let mut tuple = f.debug_tuple("Node");
        tuple.field(&node.id()).field(&node.role());

        if let Some(name) = node.name() {
            tuple.field(&name);
        }

        tuple.finish()
    }
}

#[extension(pub trait NodeExt)]
impl<'a> Node<'a> {
    fn query_by<'b>(&'b self, f: impl Fn(&Node<'_>) -> bool + 'a) -> Option<Node<'a>> {
        let mut iter = self.filtered_children(move |node: &Node<'_>| {
            if f(node) {
                FilterResult::Include
            } else {
                FilterResult::ExcludeNode
            }
        });
        let result = iter.next();

        if let Some(second) = iter.next() {
            let first = result?;
            panic!(
                "Found two or more nodes matching the query: {:?} {:?}",
                DebugNode(&first),
                DebugNode(&second)
            );
        }

        result
    }

    fn get_by<'b>(&'b self, f: impl Fn(&Node<'_>) -> bool + 'a) -> Node<'a> {
        self.query_by(f).expect("No node found matching the query")
    }

    fn query_by_name<'b>(&'b self, name: &'a str) -> Option<Node<'a>> {
        self.query_by(move |node| node.name().as_deref() == Some(name))
    }

    fn get_by_name<'b>(&'b self, name: &'a str) -> Node<'a> {
        self.query_by_name(name)
            .expect("No node found with the given name")
    }

    fn query_by_role<'b>(&'b self, role: accesskit::Role) -> Option<Node<'a>> {
        self.query_by(move |node| node.role() == role)
    }

    fn get_by_role<'b>(&'b self, role: accesskit::Role) -> Node<'a> {
        self.query_by_role(role)
            .expect("No node found with the given role")
    }
}
