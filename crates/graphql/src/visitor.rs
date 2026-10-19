//! Port of graphql-js `language/visitor`: `visit` and `visitInParallel`.
//!
//! Two traversals share graphql-js's order (`QueryDocumentKeys`, which is the
//! field order of the AST structs) and its `key`/`parent`/`path`/`ancestors`
//! bookkeeping:
//!
//! * [`visit`] is read-only. Callbacks get nodes borrowed for the document's
//!   lifetime `'a`, so a visitor may keep references into the document, and a
//!   mutable context `C` shared by all callbacks (for example a validation
//!   context shared by rules running under [`visit_in_parallel`]).
//! * [`visit_edit`] is graphql-js `visit` with callbacks that return
//!   replacement nodes ([`EditAction::Replace`]), delete nodes
//!   ([`EditAction::Delete`], JS `null`), skip (`false`) or stop (`BREAK`).
//!   The input is not modified; the edited tree is returned.
//!
//! A TypeScript visitor object `{ FieldDefinition(node) {..}, Directive: { leave(node) {..} } }`
//! ports to a `match node { NodeRef::FieldDefinition(node) => .., _ => {} }` in
//! [`Visitor::enter`] and one on `NodeRef::Directive` in [`Visitor::leave`].

use crate::ast::{AsNodeRef, FromNode, ListRef, Node, NodeRef, SlotRef};

/// One entry of graphql-js `path`: a field key or a list index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathKey {
    /// A graphql-js field name, e.g. `"selectionSet"`.
    Key(&'static str),
    Index(usize),
}

/// One entry of graphql-js `ancestors` (or the `parent`): a node or a list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ancestor<'a> {
    Node(NodeRef<'a>),
    List(ListRef<'a>),
}

impl<'a> Ancestor<'a> {
    pub fn as_node(self) -> Option<NodeRef<'a>> {
        match self {
            Ancestor::Node(n) => Some(n),
            Ancestor::List(_) => None,
        }
    }

    pub fn as_list(self) -> Option<ListRef<'a>> {
        match self {
            Ancestor::List(l) => Some(l),
            Ancestor::Node(_) => None,
        }
    }
}

/// The graphql-js visit function arguments besides the node:
/// `(node, key, parent, path, ancestors)`.
#[derive(Clone, Copy, Debug)]
pub struct VisitInfo<'a, 'v> {
    /// The key of the node in `parent`; `None` for the root.
    pub key: Option<PathKey>,
    /// The node or list holding the node; `None` for the root.
    pub parent: Option<Ancestor<'a>>,
    /// Keys from the root to the node, including `key`.
    pub path: &'v [PathKey],
    /// Nodes and lists from the root down to, but excluding, `parent`.
    pub ancestors: &'v [Ancestor<'a>],
    node_id: u64,
}

impl VisitInfo<'_, '_> {
    /// Identity of the visited node object. A leave callback sees the same id
    /// as the matching enter callback unless the node was replaced or its
    /// children were edited, in which case graphql-js passes a different object.
    pub fn node_id(&self) -> u64 {
        self.node_id
    }
}

/// What a read-only visit callback asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VisitAction {
    /// JS `undefined`.
    #[default]
    Continue,
    /// JS `false`: from `enter`, do not visit the node's children (and do not
    /// call `leave` for it).
    Skip,
    /// JS `BREAK`: stop the traversal.
    Break,
}

/// A read-only visitor; see the module docs.
pub trait Visitor<'a, C: ?Sized = ()> {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        let _ = (node, cx, info);
        VisitAction::Continue
    }

    fn leave(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        let _ = (node, cx, info);
        VisitAction::Continue
    }
}

impl<'a, C: ?Sized, V: Visitor<'a, C> + ?Sized> Visitor<'a, C> for &mut V {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        (**self).enter(node, cx, info)
    }
    fn leave(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        (**self).leave(node, cx, info)
    }
}

impl<'a, C: ?Sized, V: Visitor<'a, C> + ?Sized> Visitor<'a, C> for Box<V> {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        (**self).enter(node, cx, info)
    }
    fn leave(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        (**self).leave(node, cx, info)
    }
}

/// graphql-js `visit` for visitors that do not edit. Returns `true` when a
/// callback returned [`VisitAction::Break`].
pub fn visit<'a, C: ?Sized, V: Visitor<'a, C> + ?Sized>(
    root: impl Into<NodeRef<'a>>,
    visitor: &mut V,
    cx: &mut C,
) -> bool {
    let mut walker = Walker {
        visitor,
        cx,
        path: Vec::new(),
        ancestors: Vec::new(),
        next_id: 0,
    };
    walker.walk(root.into(), None, None)
}

struct Walker<'a, 'w, V: ?Sized, C: ?Sized> {
    visitor: &'w mut V,
    cx: &'w mut C,
    path: Vec<PathKey>,
    ancestors: Vec<Ancestor<'a>>,
    next_id: u64,
}

impl<'a, V: Visitor<'a, C> + ?Sized, C: ?Sized> Walker<'a, '_, V, C> {
    /// Returns `true` on break.
    fn walk(
        &mut self,
        node: NodeRef<'a>,
        key: Option<PathKey>,
        parent: Option<Ancestor<'a>>,
    ) -> bool {
        let node_id = self.next_id;
        self.next_id += 1;
        let info = VisitInfo {
            key,
            parent,
            path: &self.path,
            ancestors: &self.ancestors,
            node_id,
        };
        match self.visitor.enter(node, self.cx, &info) {
            VisitAction::Break => return true,
            VisitAction::Skip => return false,
            VisitAction::Continue => {}
        }
        if let Some(p) = parent {
            self.ancestors.push(p);
        }
        let children = node.children();
        for &(name, slot) in children.as_slice() {
            match slot {
                SlotRef::Absent => {}
                SlotRef::One(child) => {
                    self.path.push(PathKey::Key(name));
                    if self.walk(child, Some(PathKey::Key(name)), Some(Ancestor::Node(node))) {
                        return true;
                    }
                    self.path.pop();
                }
                SlotRef::Many(list) => {
                    self.path.push(PathKey::Key(name));
                    self.ancestors.push(Ancestor::Node(node));
                    for i in 0..list.len() {
                        self.path.push(PathKey::Index(i));
                        if self.walk(
                            list.get(i).unwrap(),
                            Some(PathKey::Index(i)),
                            Some(Ancestor::List(list)),
                        ) {
                            return true;
                        }
                        self.path.pop();
                    }
                    self.ancestors.pop();
                    self.path.pop();
                }
            }
        }
        if parent.is_some() {
            self.ancestors.pop();
        }
        let info = VisitInfo {
            key,
            parent,
            path: &self.path,
            ancestors: &self.ancestors,
            node_id,
        };
        self.visitor.leave(node, self.cx, &info) == VisitAction::Break
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkipState {
    Active,
    Skipping(u64),
    Broken,
}

/// graphql-js `visitInParallel` for read-only visitors.
pub struct ParallelVisitor<'a, 'v, C: ?Sized> {
    visitors: Vec<Box<dyn Visitor<'a, C> + 'v>>,
    skipping: Vec<SkipState>,
}

/// graphql-js `visitInParallel`: runs the visitors in one traversal. A visitor
/// that skips a node is not called again until that node is left; a visitor
/// that breaks is not called again. The merged visitor never breaks.
pub fn visit_in_parallel<'a, 'v, C: ?Sized>(
    visitors: Vec<Box<dyn Visitor<'a, C> + 'v>>,
) -> ParallelVisitor<'a, 'v, C> {
    let skipping = vec![SkipState::Active; visitors.len()];
    ParallelVisitor { visitors, skipping }
}

impl<'a, C: ?Sized> Visitor<'a, C> for ParallelVisitor<'a, '_, C> {
    fn enter(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        for (visitor, state) in self.visitors.iter_mut().zip(self.skipping.iter_mut()) {
            if *state == SkipState::Active {
                match visitor.enter(node, cx, info) {
                    VisitAction::Skip => *state = SkipState::Skipping(info.node_id),
                    VisitAction::Break => *state = SkipState::Broken,
                    VisitAction::Continue => {}
                }
            }
        }
        VisitAction::Continue
    }

    fn leave(&mut self, node: NodeRef<'a>, cx: &mut C, info: &VisitInfo<'a, '_>) -> VisitAction {
        for (visitor, state) in self.visitors.iter_mut().zip(self.skipping.iter_mut()) {
            match *state {
                SkipState::Active => {
                    if visitor.leave(node, cx, info) == VisitAction::Break {
                        *state = SkipState::Broken;
                    }
                }
                SkipState::Skipping(id) if id == info.node_id => *state = SkipState::Active,
                _ => {}
            }
        }
        VisitAction::Continue
    }
}

/// What an editing visit callback asks for.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum EditAction {
    /// JS `undefined`.
    #[default]
    Continue,
    /// JS `false`. From `enter`: skip the node's children and its `leave`.
    /// From `leave`: like graphql-js, edits made inside the node are dropped.
    Skip,
    /// JS `BREAK`.
    Break,
    /// A replacement node. From `enter`, the replacement's children are
    /// visited next, then `leave` is called with it.
    Replace(Node),
    /// JS `null`: remove the node from its list, or clear an optional field.
    Delete,
}

/// An editing visitor; see the module docs. Nodes are borrowed only for the
/// duration of the call because they may be replacements made during the
/// visit.
pub trait EditVisitor {
    fn enter(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        let _ = (node, info);
        EditAction::Continue
    }

    fn leave(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        let _ = (node, info);
        EditAction::Continue
    }
}

impl<V: EditVisitor + ?Sized> EditVisitor for &mut V {
    fn enter(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        (**self).enter(node, info)
    }
    fn leave(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        (**self).leave(node, info)
    }
}

impl<V: EditVisitor + ?Sized> EditVisitor for Box<V> {
    fn enter(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        (**self).enter(node, info)
    }
    fn leave(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        (**self).leave(node, info)
    }
}

/// The value graphql-js `visit` returns.
#[derive(Clone, Debug, PartialEq)]
pub enum EditResult {
    /// The (possibly edited) root, or whatever replaced it.
    Node(Node),
    /// The root was deleted (JS `null`).
    Deleted,
    /// Only after [`EditAction::Break`]: graphql-js then returns the last
    /// edit recorded at the depth where the traversal stopped, which can be
    /// a list.
    List(Vec<Node>),
}

impl EditResult {
    /// The result node; panics when there is none.
    pub fn into_node(self) -> Node {
        match self {
            EditResult::Node(n) => n,
            other => panic!("visit_edit did not produce a node: {other:?}"),
        }
    }

    /// The result as a node of type `T`; panics when it is something else.
    pub fn into_kind<T: FromNode>(self) -> T {
        match T::from_node(self.into_node()) {
            Ok(t) => t,
            Err(n) => panic!("visit_edit produced a {} node", n.kind()),
        }
    }

    /// The result as a `Document`; panics when it is something else.
    pub fn into_document(self) -> crate::ast::Document {
        self.into_kind()
    }
}

/// graphql-js `visit` with an editing visitor. The input is left untouched;
/// nodes outside edited paths are cloned into the result.
///
/// Panics when an edit cannot be represented in the typed AST: deleting a
/// required field (e.g. a `Field`'s `name`) or replacing a node with a node
/// of a kind its field cannot hold (e.g. a `Directive` with a `Name`).
pub fn visit_edit<'a, V: EditVisitor + ?Sized>(
    root: impl Into<NodeRef<'a>>,
    visitor: &mut V,
) -> EditResult {
    let root = root.into();
    let mut walker = EditWalker {
        visitor,
        path: Vec::new(),
        next_id: 0,
    };
    let mut ancestors = Vec::new();
    let last = match walker.walk(root, None, None, &mut ancestors) {
        Ok(mut edits) => edits.pop(),
        Err(Break::Fresh(mut edits)) => edits.pop(),
        Err(Break::Resolved(last)) => last.map(|b| *b),
    };
    match last {
        None => EditResult::Node(root.to_owned_node()),
        Some(EditVal::Node(n)) => EditResult::Node(n),
        Some(EditVal::Null) => EditResult::Deleted,
        Some(EditVal::List(l)) => EditResult::List(l),
    }
}

/// A value graphql-js records in its `edits` lists.
#[derive(Debug)]
pub(crate) enum EditVal {
    Node(Node),
    Null,
    /// An edited copy of a list.
    List(Vec<Node>),
}

enum Break {
    /// Returned by the node whose callback broke, with the edits that node
    /// had recorded in its parent's list before breaking.
    Fresh(Vec<EditVal>),
    /// The last edit at the depth where the traversal stopped.
    Resolved(Option<Box<EditVal>>),
}

struct EditWalker<'w, V: ?Sized> {
    visitor: &'w mut V,
    path: Vec<PathKey>,
    next_id: u64,
}

impl<V: EditVisitor + ?Sized> EditWalker<'_, V> {
    /// Visits `node`; returns the edits to record for it in its parent's list.
    fn walk<'b>(
        &mut self,
        node: NodeRef<'b>,
        key: Option<PathKey>,
        parent: Option<Ancestor<'b>>,
        ancestors: &mut Vec<Ancestor<'b>>,
    ) -> Result<Vec<EditVal>, Break> {
        let node_id = self.next_id;
        self.next_id += 1;
        let info = VisitInfo {
            key,
            parent,
            path: &self.path,
            ancestors: ancestors.as_slice(),
            node_id,
        };
        match self.visitor.enter(node, &info) {
            EditAction::Break => Err(Break::Fresh(Vec::new())),
            EditAction::Skip => Ok(Vec::new()),
            EditAction::Delete => Ok(vec![EditVal::Null]),
            EditAction::Continue => {
                self.walk_children_and_leave(node, key, parent, ancestors, node_id)
            }
            EditAction::Replace(replacement) => {
                // The replacement lives only in this frame, so its subtree is
                // walked with a copy of the ancestor list of shorter lifetime.
                let mut inner: Vec<Ancestor<'_>> = ancestors.clone();
                let result = self.walk_children_and_leave(
                    replacement.as_ref(),
                    key,
                    parent,
                    &mut inner,
                    node_id,
                );
                match result {
                    Ok(rest) => {
                        drop(inner);
                        let mut edits = vec![EditVal::Node(replacement)];
                        edits.extend(rest);
                        Ok(edits)
                    }
                    Err(Break::Fresh(rest)) => {
                        drop(inner);
                        let mut edits = vec![EditVal::Node(replacement)];
                        edits.extend(rest);
                        Err(Break::Fresh(edits))
                    }
                    Err(resolved) => Err(resolved),
                }
            }
        }
    }

    fn walk_children_and_leave<'c>(
        &mut self,
        node: NodeRef<'c>,
        key: Option<PathKey>,
        parent: Option<Ancestor<'c>>,
        ancestors: &mut Vec<Ancestor<'c>>,
        node_id: u64,
    ) -> Result<Vec<EditVal>, Break> {
        if let Some(p) = parent {
            ancestors.push(p);
        }
        let mut edits: Vec<(usize, EditVal)> = Vec::new();
        let children = node.children();
        for (index, &(name, slot)) in children.as_slice().iter().enumerate() {
            match slot {
                SlotRef::Absent => {}
                SlotRef::One(child) => {
                    self.path.push(PathKey::Key(name));
                    let result = self.walk(
                        child,
                        Some(PathKey::Key(name)),
                        Some(Ancestor::Node(node)),
                        ancestors,
                    );
                    self.path.pop();
                    match result {
                        Ok(child_edits) => {
                            edits.extend(child_edits.into_iter().map(|e| (index, e)))
                        }
                        Err(Break::Fresh(child_edits)) => {
                            edits.extend(child_edits.into_iter().map(|e| (index, e)));
                            return Err(Break::Resolved(edits.pop().map(|(_, e)| Box::new(e))));
                        }
                        Err(resolved) => return Err(resolved),
                    }
                }
                SlotRef::Many(list) => {
                    self.path.push(PathKey::Key(name));
                    ancestors.push(Ancestor::Node(node));
                    let mut list_edits: Vec<(usize, EditVal)> = Vec::new();
                    for i in 0..list.len() {
                        self.path.push(PathKey::Index(i));
                        let result = self.walk(
                            list.get(i).unwrap(),
                            Some(PathKey::Index(i)),
                            Some(Ancestor::List(list)),
                            ancestors,
                        );
                        self.path.pop();
                        match result {
                            Ok(item_edits) => {
                                list_edits.extend(item_edits.into_iter().map(|e| (i, e)))
                            }
                            Err(Break::Fresh(item_edits)) => {
                                list_edits.extend(item_edits.into_iter().map(|e| (i, e)));
                                return Err(Break::Resolved(
                                    list_edits.pop().map(|(_, e)| Box::new(e)),
                                ));
                            }
                            Err(resolved) => return Err(resolved),
                        }
                    }
                    ancestors.pop();
                    self.path.pop();
                    if !list_edits.is_empty() {
                        edits.push((index, EditVal::List(rebuild_list(list, list_edits))));
                    }
                }
            }
        }
        if parent.is_some() {
            ancestors.pop();
        }
        let rebuilt = if edits.is_empty() {
            None
        } else {
            Some(node.rebuild(edits))
        };
        let leave_id = if rebuilt.is_some() {
            let id = self.next_id;
            self.next_id += 1;
            id
        } else {
            node_id
        };
        let info = VisitInfo {
            key,
            parent,
            path: &self.path,
            ancestors: ancestors.as_slice(),
            node_id: leave_id,
        };
        let leave_node = rebuilt.as_ref().map_or(node, |n| n.as_ref());
        match self.visitor.leave(leave_node, &info) {
            EditAction::Break => Err(Break::Fresh(Vec::new())),
            // graphql-js records nothing when leave returns `false`, which
            // drops the edited copy.
            EditAction::Skip => Ok(Vec::new()),
            EditAction::Delete => Ok(vec![EditVal::Null]),
            EditAction::Replace(replacement) => Ok(vec![EditVal::Node(replacement)]),
            EditAction::Continue => Ok(rebuilt.map(EditVal::Node).into_iter().collect()),
        }
    }
}

/// Applies item edits (in index order, last edit per index wins, `Null`
/// removes) to a copy of `list`.
fn rebuild_list(list: ListRef<'_>, edits: Vec<(usize, EditVal)>) -> Vec<Node> {
    let mut out = Vec::with_capacity(list.len());
    let mut edits = edits.into_iter().peekable();
    for i in 0..list.len() {
        let mut last = None;
        while let Some((j, _)) = edits.peek() {
            if *j != i {
                break;
            }
            last = Some(edits.next().unwrap().1);
        }
        match last {
            None => out.push(list.get(i).unwrap().to_owned_node()),
            Some(EditVal::Node(n)) => out.push(n),
            Some(EditVal::Null) => {}
            Some(EditVal::List(_)) => unreachable!("list items are nodes"),
        }
    }
    out
}

fn incompatible(node: &Node, kind: &str, key: &str) -> ! {
    panic!(
        "visit_edit: a {} node cannot be stored in {kind}.{key}",
        node.kind()
    )
}

pub(crate) fn edit_one<T: FromNode>(v: EditVal, kind: &str, key: &str) -> T {
    match v {
        EditVal::Node(n) => T::from_node(n).unwrap_or_else(|n| incompatible(&n, kind, key)),
        EditVal::Null => panic!("visit_edit: cannot delete required field {kind}.{key}"),
        EditVal::List(_) => unreachable!("{kind}.{key} is not a list"),
    }
}

pub(crate) fn edit_opt<T: FromNode>(v: EditVal, kind: &str, key: &str) -> Option<T> {
    match v {
        EditVal::Node(n) => Some(T::from_node(n).unwrap_or_else(|n| incompatible(&n, kind, key))),
        EditVal::Null => None,
        EditVal::List(_) => unreachable!("{kind}.{key} is not a list"),
    }
}

pub(crate) fn edit_scalar<T>(_: EditVal) -> T {
    unreachable!("scalar fields are not visited, so never edited")
}

pub(crate) fn edit_list<T: FromNode>(v: EditVal, kind: &str, key: &str) -> Vec<T> {
    match v {
        EditVal::List(items) => items
            .into_iter()
            .map(|n| T::from_node(n).unwrap_or_else(|n| incompatible(&n, kind, key)))
            .collect(),
        _ => unreachable!("{kind}.{key} is a list"),
    }
}

/// graphql-js `visitInParallel` for editing visitors.
pub struct ParallelEditVisitor<'v> {
    visitors: Vec<Box<dyn EditVisitor + 'v>>,
    skipping: Vec<SkipState>,
}

/// graphql-js `visitInParallel` for [`visit_edit`]. The first visitor that
/// returns a replacement or deletion for a node decides the edit, and the
/// visitors after it are not called for that node.
pub fn visit_in_parallel_edit<'v>(
    visitors: Vec<Box<dyn EditVisitor + 'v>>,
) -> ParallelEditVisitor<'v> {
    let skipping = vec![SkipState::Active; visitors.len()];
    ParallelEditVisitor { visitors, skipping }
}

impl EditVisitor for ParallelEditVisitor<'_> {
    fn enter(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        for (visitor, state) in self.visitors.iter_mut().zip(self.skipping.iter_mut()) {
            if *state == SkipState::Active {
                match visitor.enter(node, info) {
                    EditAction::Skip => *state = SkipState::Skipping(info.node_id),
                    EditAction::Break => *state = SkipState::Broken,
                    EditAction::Continue => {}
                    edit => return edit,
                }
            }
        }
        EditAction::Continue
    }

    fn leave(&mut self, node: NodeRef<'_>, info: &VisitInfo<'_, '_>) -> EditAction {
        for (visitor, state) in self.visitors.iter_mut().zip(self.skipping.iter_mut()) {
            match *state {
                SkipState::Active => match visitor.leave(node, info) {
                    EditAction::Break => *state = SkipState::Broken,
                    EditAction::Continue | EditAction::Skip => {}
                    edit => return edit,
                },
                SkipState::Skipping(id) if id == info.node_id => *state = SkipState::Active,
                _ => {}
            }
        }
        EditAction::Continue
    }
}

/// Lets `&T` for any AST type be used where a [`NodeRef`] is expected.
impl<'a, T: AsNodeRef> From<&'a Box<T>> for NodeRef<'a> {
    fn from(x: &'a Box<T>) -> NodeRef<'a> {
        x.as_node_ref()
    }
}
