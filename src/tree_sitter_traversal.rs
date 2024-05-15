//! Modified from https://github.com/skmendez/tree-sitter-traversal/blob/79ae415626c929e1c7b7c7c3a162ed45c21710bd/src/lib.rs
//!
//! I have to copy this file because it's tree-sitter dependency is outdated.
//!
//! MIT License
//!
//! Copyright (c) 2021 Sebastian Mendez
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
//!
//! Iterators to traverse tree-sitter [`Tree`]s using a [`TreeCursor`],
//! with a [`Cursor`] trait to allow for traversing arbitrary n-ary trees.
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```
//! # #[cfg(feature = "tree-sitter")]
//! # {
//! use tree_sitter::{Node, Tree};
//! use std::collections::HashSet;
//! use std::iter::FromIterator;
//!
//! use tree_sitter_traversal::{traverse, traverse_tree, Order};
//! # fn get_tree() -> Tree {
//! #     use tree_sitter::Parser;
//! #     let mut parser = Parser::new();
//! #     let lang = tree_sitter_rust::language();
//! #     parser.set_language(lang).expect("Error loading Rust grammar");
//! #     return parser.parse("fn double(x: usize) -> usize { x * 2 }", None).expect("Error parsing provided code");
//! # }
//!
//! // Non-existent method, imagine it gets a valid Tree with >1 node
//! let tree: Tree = get_tree();
//! let preorder: Vec<Node<'_>> = traverse(tree.walk(), Order::Pre).collect::<Vec<_>>();
//! let postorder: Vec<Node<'_>> = traverse_tree(&tree, Order::Post).collect::<Vec<_>>();
//! // For any tree with more than just a root node,
//! // the order of preorder and postorder will be different
//! assert_ne!(preorder, postorder);
//! // However, they will have the same amount of nodes
//! assert_eq!(preorder.len(), postorder.len());
//! // Specifically, they will have the exact same nodes, just in a different order
//! assert_eq!(
//!     <HashSet<_>>::from_iter(preorder.into_iter()),
//!     <HashSet<_>>::from_iter(postorder.into_iter())
//! );
//! # }
//! ```
//!
//! [`Tree`]: tree_sitter::Tree
//! [`TreeCursor`]: tree_sitter::TreeCursor
//! [`Cursor`]: crate::Cursor

use core::iter::FusedIterator;

/// Trait which represents a stateful cursor in a n-ary tree.
/// The cursor can be moved between nodes in the tree by the given methods,
/// and the node which the cursor is currently pointing at can be read as well.
pub trait Cursor {
    /// The type of the nodes which the cursor points at; the cursor is always pointing
    /// at exactly one of this type.
    type Node;

    /// Move this cursor to the first child of its current node.
    ///
    /// This returns `true` if the cursor successfully moved, and returns `false`
    /// if there were no children.
    fn goto_first_child(&mut self) -> bool;

    /// Move this cursor to the parent of its current node.
    ///
    /// This returns `true` if the cursor successfully moved, and returns `false`
    /// if there was no parent node (the cursor was already on the root node).
    fn goto_parent(&mut self) -> bool;

    /// Move this cursor to the next sibling of its current node.
    ///
    /// This returns `true` if the cursor successfully moved, and returns `false`
    /// if there was no next sibling node.
    fn goto_next_sibling(&mut self) -> bool;

    /// Get the node which the cursor is currently pointing at.
    fn node(&self) -> Self::Node;
}

impl<'a, T> Cursor for &'a mut T
where
    T: Cursor,
{
    type Node = T::Node;

    fn goto_first_child(&mut self) -> bool {
        T::goto_first_child(self)
    }

    fn goto_parent(&mut self) -> bool {
        T::goto_parent(self)
    }

    fn goto_next_sibling(&mut self) -> bool {
        T::goto_next_sibling(self)
    }

    fn node(&self) -> Self::Node {
        T::node(self)
    }
}

/// Quintessential implementation of [`Cursor`] for tree-sitter's [`TreeCursor`]
///
/// [`TreeCursor`]: tree_sitter::TreeCursor
/// [`Cursor`]: crate::Cursor
impl<'a> Cursor for tree_sitter::TreeCursor<'a> {
    type Node = tree_sitter::Node<'a>;

    fn goto_first_child(&mut self) -> bool {
        self.goto_first_child()
    }

    fn goto_parent(&mut self) -> bool {
        self.goto_parent()
    }

    fn goto_next_sibling(&mut self) -> bool {
        self.goto_next_sibling()
    }

    fn node(&self) -> Self::Node {
        self.node()
    }
}

/// Order to iterate through a n-ary tree; for n-ary trees only
/// Pre-order and Post-order make sense.
#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub(crate) enum Order {
    Pre,
    Post,
}

/// Iterative traversal of the tree; serves as a reference for both
/// PreorderTraversal and PostorderTraversal, as they both will call the exact same
/// cursor methods in the exact same order as this function for a given tree; the order
/// is also the same as traverse_recursive.
#[allow(dead_code)]
fn traverse_iterative<C: Cursor, F>(mut c: C, order: Order, mut cb: F)
where
    F: FnMut(C::Node),
{
    loop {
        // This is the first time we've encountered the node, so we'll call if preorder
        if order == Order::Pre {
            cb(c.node());
        }

        // Keep travelling down the tree as far as we can
        if c.goto_first_child() {
            continue;
        }

        let node = c.node();

        // If we can't travel any further down, try going to next sibling and repeating
        if c.goto_next_sibling() {
            // If we succeed in going to the previous nodes sibling,
            // we won't be encountering that node again, so we'll call if postorder
            if order == Order::Post {
                cb(node);
            }
            continue;
        }

        // Otherwise, we must travel back up; we'll loop until we reach the root or can
        // go to the next sibling of a node again.
        loop {
            // Since we're retracing back up the tree, this is the last time we'll encounter
            // this node, so we'll call if postorder
            if order == Order::Post {
                cb(c.node());
            }
            if !c.goto_parent() {
                // We have arrived back at the root, so we are done.
                return;
            }

            let node = c.node();

            if c.goto_next_sibling() {
                // If we succeed in going to the previous node's sibling,
                // we will go back to travelling down that sibling's tree, and we also
                // won't be encountering the previous node again, so we'll call if postorder
                if order == Order::Post {
                    cb(node);
                }
                break;
            }
        }
    }
}

/// Idiomatic recursive traversal of the tree; this version is easier to understand
/// conceptually, but the recursion is actually unnecessary and can cause stack overflow.
#[allow(dead_code)]
fn traverse_recursive<C: Cursor, F>(mut c: C, order: Order, mut cb: F)
where
    F: FnMut(C::Node),
{
    traverse_helper(&mut c, order, &mut cb);
}

fn traverse_helper<C: Cursor, F>(c: &mut C, order: Order, cb: &mut F)
where
    F: FnMut(C::Node),
{
    // If preorder, call the callback when we first touch the node
    if order == Order::Pre {
        cb(c.node());
    }
    if c.goto_first_child() {
        // If there is a child, recursively call on
        // that child and all its siblings
        loop {
            traverse_helper(c, order, cb);
            if !c.goto_next_sibling() {
                break;
            }
        }
        // Make sure to reset back to the original node;
        // this must always return true, as we only get here if we go to a child
        // of the original node.
        assert!(c.goto_parent());
    }
    // If preorder, call the callback after the recursive calls on child nodes
    if order == Order::Post {
        cb(c.node());
    }
}

struct PreorderTraverse<C> {
    cursor: Option<C>,
}

impl<C> PreorderTraverse<C> {
    pub(crate) fn new(c: C) -> Self {
        PreorderTraverse { cursor: Some(c) }
    }
}

impl<C> Iterator for PreorderTraverse<C>
where
    C: Cursor,
{
    type Item = C::Node;

    fn next(&mut self) -> Option<Self::Item> {
        let c = match self.cursor.as_mut() {
            None => {
                return None;
            }
            Some(c) => c,
        };

        // We will always return the node we were on at the start;
        // the node we traverse to will either be returned on the next iteration,
        // or will be back to the root node, at which point we'll clear out
        // the reference to the cursor
        let node = c.node();

        // First, try to go to a child or a sibling; if either succeed, this will be the
        // first time we touch that node, so it'll be the next starting node
        if c.goto_first_child() || c.goto_next_sibling() {
            return Some(node);
        }

        loop {
            // If we can't go to the parent, then that means we've reached the root, and our
            // iterator will be done in the next iteration
            if !c.goto_parent() {
                self.cursor = None;
                break;
            }

            // If we get to a sibling, then this will be the first time we touch that node,
            // so it'll be the next starting node
            if c.goto_next_sibling() {
                break;
            }
        }

        Some(node)
    }
}

struct PostorderTraverse<C> {
    cursor: Option<C>,
    retracing: bool,
}

impl<C> PostorderTraverse<C> {
    pub(crate) fn new(c: C) -> Self {
        PostorderTraverse {
            cursor: Some(c),
            retracing: false,
        }
    }
}

impl<C> Iterator for PostorderTraverse<C>
where
    C: Cursor,
{
    type Item = C::Node;

    fn next(&mut self) -> Option<Self::Item> {
        let c = match self.cursor.as_mut() {
            None => {
                return None;
            }
            Some(c) => c,
        };

        // For the postorder traversal, we will only return a node when we are travelling back up
        // the tree structure. Therefore, we go all the way to the leaves of the tree immediately,
        // and only when we are retracing do we return elements
        if !self.retracing {
            while c.goto_first_child() {}
        }

        // Much like in preorder traversal, we want to return the node we were previously at.
        // We know this will be the last time we touch this node, as we will either be going
        // to its next sibling or retracing back up the tree
        let node = c.node();
        if c.goto_next_sibling() {
            // If we successfully go to a sibling of this node, we want to go back down
            // the tree on the next iteration
            self.retracing = false;
        } else {
            // If we weren't already retracing, we are now; travel upwards until we can
            // go to the next sibling or reach the root again
            self.retracing = true;
            if !c.goto_parent() {
                // We've reached the root again, and our iteration is done
                self.cursor = None;
            }
        }

        Some(node)
    }
}

// Used for visibility purposes, in case this struct becomes public
struct Traverse<C> {
    inner: TraverseInner<C>,
}

enum TraverseInner<C> {
    Post(PostorderTraverse<C>),
    Pre(PreorderTraverse<C>),
}

impl<C> Traverse<C> {
    pub(crate) fn new(c: C, order: Order) -> Self {
        let inner = match order {
            Order::Pre => TraverseInner::Pre(PreorderTraverse::new(c)),
            Order::Post => TraverseInner::Post(PostorderTraverse::new(c)),
        };
        Self { inner }
    }
}

#[cfg(feature = "tree-sitter")]
impl<'a> Traverse<tree_sitter::TreeCursor<'a>> {
    #[allow(dead_code)]
    pub(crate) fn from_tree(tree: &'a tree_sitter::Tree, order: Order) -> Self {
        Traverse::new(tree.walk(), order)
    }
}

/// Convenience method to traverse a tree-sitter [`Tree`] in an order according to `order`.
///
/// [`Tree`]: tree_sitter::Tree
#[cfg(feature = "tree-sitter")]
pub(crate) fn traverse_tree(
    tree: &tree_sitter::Tree,
    order: Order,
) -> impl FusedIterator<Item = tree_sitter::Node> {
    return traverse(tree.walk(), order);
}

/// Traverse an n-ary tree using `cursor`, returning the nodes of the tree through an iterator
/// in an order according to `order`.
///
/// `cursor` must be at the root of the tree
/// (i.e. `cursor.goto_parent()` must return false)
pub(crate) fn traverse<C: Cursor>(
    mut cursor: C,
    order: Order,
) -> impl FusedIterator<Item = C::Node> {
    assert!(!cursor.goto_parent());
    Traverse::new(cursor, order)
}

impl<C> Iterator for Traverse<C>
where
    C: Cursor,
{
    type Item = C::Node;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            TraverseInner::Post(ref mut i) => i.next(),
            TraverseInner::Pre(ref mut i) => i.next(),
        }
    }
}

// We know that PreorderTraverse and PostorderTraverse are fused due to their implementation,
// so we can add this bound for free.
impl<C> FusedIterator for Traverse<C> where C: Cursor {}

#[cfg(test)]
#[cfg(feature = "tree-sitter")]
mod tree_sitter_tests {
    use super::*;

    extern crate std;
    use std::vec::Vec;
    use tree_sitter::{Parser, Tree};

    const EX1: &str = r#"
fn double(x: usize) -> usize {
    return 2 * x;
}"#;

    const EX2: &str = r#"
// Intentionally invalid code below

"123

const DOUBLE = 2;

function double(x: usize) -> usize {
    return DOUBLE * x;
}"#;

    const EX3: &str = "";

    /// For a given tree and iteration order, verify that the two callback approaches
    /// and the Iterator approach are all equivalent
    fn generate_traversals(tree: &Tree, order: Order) {
        let mut recursive_callback = Vec::new();
        traverse_recursive(tree.walk(), order, |n| recursive_callback.push(n));
        let mut iterative_callback = Vec::new();
        traverse_iterative(tree.walk(), order, |n| iterative_callback.push(n));
        let iterator = traverse(tree.walk(), order).collect::<Vec<_>>();

        assert_eq!(recursive_callback, iterative_callback);
        assert_eq!(iterative_callback, iterator);
    }

    /// Helper function to generate a Tree from Rust code
    fn get_tree(code: &str) -> Tree {
        let mut parser = Parser::new();
        let lang = tree_sitter_rust::language();
        parser
            .set_language(lang)
            .expect("Error loading Rust grammar");
        return parser
            .parse(code, None)
            .expect("Error parsing provided code");
    }

    #[test]
    fn test_equivalence() {
        for code in [EX1, EX2, EX3] {
            let tree = get_tree(code);
            for order in [Order::Pre, Order::Post] {
                generate_traversals(&tree, order);
            }
        }
    }

    #[test]
    fn test_postconditions() {
        let parsed = get_tree(EX1);
        let mut walk = parsed.walk();
        for order in [Order::Pre, Order::Post] {
            let mut iter = traverse(&mut walk, order);
            while iter.next().is_some() {}
            // Make sure it's fused
            assert!(iter.next().is_none());
            // Really make sure it's fused
            assert!(iter.next().is_none());
            drop(iter);
            // Verify that the walk is reset to the root_node and can be reused
            assert_eq!(walk.node(), parsed.root_node());
        }
    }

    #[test]
    #[should_panic]
    fn test_panic() {
        // Tests that the precondition check works
        let parsed = get_tree(EX1);
        let mut walk = parsed.walk();
        walk.goto_first_child();
        let iter = traverse(&mut walk, Order::Pre);
        iter.count();
    }

    #[test]
    fn example() {
        use std::collections::HashSet;
        use std::iter::FromIterator;
        use tree_sitter::{Node, Tree};
        let tree: Tree = get_tree(EX1);
        let preorder: Vec<Node<'_>> = traverse(tree.walk(), Order::Pre).collect::<Vec<_>>();
        let postorder: Vec<Node<'_>> = traverse_tree(&tree, Order::Post).collect::<Vec<_>>();
        assert_ne!(preorder, postorder);
        assert_eq!(preorder.len(), postorder.len());
        assert_eq!(
            <HashSet<_>>::from_iter(preorder.into_iter()),
            <HashSet<_>>::from_iter(postorder.into_iter())
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Root;

    // Root represents a tree where there's only one node, the root, and its type is the unit type
    impl Cursor for Root {
        type Node = ();

        fn goto_first_child(&mut self) -> bool {
            false
        }

        fn goto_parent(&mut self) -> bool {
            false
        }

        fn goto_next_sibling(&mut self) -> bool {
            false
        }

        fn node(&self) -> Self::Node {}
    }

    #[test]
    fn test_root() {
        assert_eq!(1, traverse(Root, Order::Pre).count());
        assert_eq!(1, traverse(Root, Order::Post).count());
    }
}
