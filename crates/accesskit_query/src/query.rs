use crate::Node;
use accesskit_consumer::{FilterResult, Node as AKNode};
use std::iter::{FusedIterator, Peekable};

fn query_by_impl<'tree>(mut iter: impl Iterator<Item = Node<'tree>>) -> Option<Node<'tree>> {
    let result = iter.next();

    if let Some(second) = iter.next() {
        let first = result?;
        panic!(
            "Found two or more nodes matching the query: {:?} {:?}",
            first.node().id(),
            second.node().id(),
        );
    }
    result
}

pub trait Queryable<'tree, 'node> {
    fn node(&'node self) -> crate::Node<'tree>;

    fn query_all_by(
        &'node self,
        f: impl Fn(&Node<'_>) -> bool + 'tree,
    ) -> impl Iterator<Item = Node<'tree>>
           + DoubleEndedIterator<Item = Node<'tree>>
           + FusedIterator<Item = Node<'tree>>
           + 'tree {
        let root = self.node();
        let queue = root.queue();
        root.filtered_children(move |node: &AKNode<'_>| {
            if f(&Node::new(*node, queue)) {
                FilterResult::Include
            } else {
                FilterResult::ExcludeNode
            }
        })
        .map(|node| Node::new(node, queue))
    }

    fn query_by(&'node self, f: impl Fn(&Node<'_>) -> bool + 'tree) -> Option<Node<'tree>> {
        query_by_impl(self.query_all_by(f))
    }

    fn get_by(&'node self, f: impl Fn(&Node<'_>) -> bool + 'tree) -> Node<'tree> {
        self.query_by(f).expect("No node found matching the query")
    }

    fn query_all_by_name(&'node self, name: &'tree str) -> impl IterType<'tree> + 'tree {
        self.query_all_by(move |node| node.name().as_deref() == Some(name))
    }

    fn query_by_name(&'node self, name: &'tree str) -> Option<Node<'tree>> {
        query_by_impl(self.query_all_by_name(name))
    }

    fn get_by_name(&'node self, name: &'tree str) -> Node<'tree> {
        self.query_by_name(name)
            .expect("No node found with the given name")
    }

    fn query_all_by_role(&'node self, role: accesskit::Role) -> impl IterType<'tree> + 'tree {
        self.query_all_by(move |node| node.role() == role)
    }

    fn query_by_role(&'node self, role: accesskit::Role) -> Option<Node<'tree>> {
        query_by_impl(self.query_all_by_role(role))
    }

    fn get_by_role(&'node self, role: accesskit::Role) -> Node<'tree> {
        self.query_by_role(role)
            .expect("No node found with the given role")
    }
}

trait IterType<'tree>:
    Iterator<Item = Node<'tree>>
    + DoubleEndedIterator<Item = Node<'tree>>
    + FusedIterator<Item = Node<'tree>>
{
}

impl<'tree, T> IterType<'tree> for T where
    T: Iterator<Item = Node<'tree>>
        + DoubleEndedIterator<Item = Node<'tree>>
        + FusedIterator<Item = Node<'tree>>
{
}

// pub trait Findable<'tree, 'node, 's>: Queryable<'tree, 'node> {
//     fn run(&mut self);
//
//     fn find_timeout(&self) -> std::time::Duration {
//         std::time::Duration::from_secs(5)
//     }
//
//     fn find_all_by(
//         &'node mut self,
//         f: impl Fn(&Node<'_>) -> bool + Copy + 'tree,
//     ) -> impl IterType<'tree> + 'tree {
//         let timeout = self.find_timeout();
//         let step = timeout / 10;
//
//         let mut start_time = std::time::Instant::now();
//
//         loop {
//             {
//                 let node = self.node();
//                 let iter = node.query_all_by(f);
//                 let mut peekable = iter.peekable();
//                 if !peekable.peek().is_none() {
//                     return peekable;
//                 }
//
//                 if start_time.elapsed() > timeout {
//                     panic!("Timeout exceeded while waiting for node");
//                 }
//             }
//
//             std::thread::sleep(step);
//         }
//     }
// }
