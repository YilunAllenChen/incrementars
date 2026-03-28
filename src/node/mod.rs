use std::cmp::min;
use std::collections::BinaryHeap;
use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use self::traits::MaybeDirty;
use bitmap::Bitmap;
use traits::StabilizationCallback;

mod bind;
mod bitmap;
mod map;
mod map2;
mod map3;
mod mapn;
mod traits;
mod var;

pub use self::{
    bind::Bind1,
    map::Map1,
    map2::Map2,
    map3::Map3,
    mapn::MapN,
    traits::{IntoInput, Observable},
    var::Var,
};

// Vars are assigned the maximum depth so that derived nodes, which subtract 1
// per level, are always processed after their inputs in the stabilization queue.
const VAR_DEPTH: i32 = 1_000;

/// The incremental computation graph.
///
/// Create input nodes with [`var`](Incrementars::var), wire them together with
/// [`map`](Incrementars::map), [`map2`](Incrementars::map2),
/// [`map3`](Incrementars::map3), [`mapn`](Incrementars::mapn), and
/// [`bind`](Incrementars::bind), then call [`stabilize`](Incrementars::stabilize)
/// to propagate pending changes through the graph.
///
/// # Example
/// ```
/// use incrementars::prelude::*;
/// let mut dag = Incrementars::new();
/// let x = dag.var(2);
/// let y = dag.map(x.clone(), |v| v * v);
/// assert_eq!(y.observe(), 4);
/// x.set(3);
/// dag.stabilize();
/// assert_eq!(y.observe(), 9);
/// ```
pub struct Incrementars {
    // node id → node handle. IDs are assigned sequentially from 0 by next_id(),
    // and remain stable even after removal.
    nodes: Vec<Option<Rc<RefCell<dyn traits::Node>>>>,
    id_counter: usize,

    inputs: Vec<Box<dyn MaybeDirty>>,
    // parent_id → [child_ids]: which nodes depend on a given node
    pub(crate) dependencies: Vec<Vec<usize>>,
    // child_id → [parent_ids]: which nodes a given node depends on
    reverse_dependencies: Vec<Vec<usize>>,
    hooks: Vec<Vec<(usize, Box<dyn FnMut()>)>>,
    hook_counter: usize,
}

impl Incrementars {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            id_counter: 0,
            inputs: vec![],
            dependencies: vec![],
            reverse_dependencies: vec![],
            hooks: vec![],
            hook_counter: 0,
        }
    }

    fn next_id(&mut self) -> usize {
        let id = self.id_counter;
        self.id_counter += 1;
        self.nodes.push(None);
        self.dependencies.push(vec![]);
        self.reverse_dependencies.push(vec![]);
        self.hooks.push(vec![]);
        id
    }

    fn node(&self, id: usize) -> Rc<RefCell<dyn traits::Node>> {
        self.nodes[id]
            .as_ref()
            .expect("node id missing from graph")
            .clone()
    }

    /// Adds a directed edge parent → child to both dependency maps.
    /// Silently deduplicates: calling with the same pair twice has no effect.
    fn add_edge(&mut self, parent_id: usize, child_id: usize) {
        let deps = &mut self.dependencies[parent_id];
        if deps.last() != Some(&child_id) && !deps.contains(&child_id) {
            deps.push(child_id);
        }
        let rev = &mut self.reverse_dependencies[child_id];
        if rev.last() != Some(&parent_id) && !rev.contains(&parent_id) {
            rev.push(parent_id);
        }
    }

    /// Creates an input node holding `value`.
    ///
    /// Use [`Var::set`] to update the value. Changes are not visible to downstream
    /// nodes until the next call to [`stabilize`](Incrementars::stabilize).
    pub fn var<T: Clone + 'static>(&mut self, value: T) -> Var<T> {
        let id = self.next_id();
        let node = Rc::new(RefCell::new(var::_Var::new(id, VAR_DEPTH, value)));
        self.nodes[id] = Some(node.clone());
        self.inputs.push(Box::new(Var { node: node.clone() }));
        Var { node }
    }

    /// Creates a node that applies `f` to the output of `input` during stabilization.
    ///
    /// The initial value is computed eagerly when the node is created. After that,
    /// `f` is only called when `input` has changed since the last stabilization, and
    /// its result is only propagated downstream if it differs from the previous
    /// output (cutoff optimization, requires `O: PartialEq`).
    pub fn map<I: 'static, O: PartialEq + 'static>(
        &mut self,
        input: impl IntoInput<I>,
        f: impl Fn(I) -> O + 'static,
    ) -> Map1<I, O> {
        let input = input.into_input();
        let id = self.next_id();
        let input_id = input.id();
        self.add_edge(input_id, id);
        let node = Rc::new(RefCell::new(map::_Map1 {
            id,
            depth: input.depth() - 1,
            value: f(input.observe()),
            input: Some(input),
            f: Some(Box::new(f)),
        }));
        self.nodes[id] = Some(node.clone());
        Map1 { node }
    }

    /// Creates a node that applies `f` to the outputs of `input1` and `input2`
    /// during stabilization.
    ///
    /// The initial value is computed eagerly when the node is created. After that,
    /// `f` is called when either input has changed, and the result is only
    /// propagated if it differs from the previous output (cutoff optimization).
    pub fn map2<I1: 'static, I2: 'static, O: PartialEq + 'static>(
        &mut self,
        input1: impl IntoInput<I1>,
        input2: impl IntoInput<I2>,
        f: impl Fn(I1, I2) -> O + 'static,
    ) -> Map2<I1, I2, O> {
        let input1 = input1.into_input();
        let input2 = input2.into_input();
        let id = self.next_id();
        let (id1, id2) = (input1.id(), input2.id());
        self.add_edge(id1, id);
        self.add_edge(id2, id);
        let node = Rc::new(RefCell::new(map2::_Map2 {
            id,
            depth: min(input1.depth(), input2.depth()) - 1,
            value: f(input1.observe(), input2.observe()),
            input1: Some(input1),
            input2: Some(input2),
            f: Some(Box::new(f)),
        }));
        self.nodes[id] = Some(node.clone());
        Map2 { node }
    }

    /// Creates a node that applies `f` to the outputs of `input1`, `input2`,
    /// and `input3` during stabilization.
    ///
    /// The initial value is computed eagerly when the node is created. After that,
    /// `f` is called when any input has changed, and the result is only propagated
    /// if it differs from the previous output (cutoff optimization).
    pub fn map3<I1: 'static, I2: 'static, I3: 'static, O: PartialEq + 'static>(
        &mut self,
        input1: impl IntoInput<I1>,
        input2: impl IntoInput<I2>,
        input3: impl IntoInput<I3>,
        f: impl Fn(I1, I2, I3) -> O + 'static,
    ) -> Map3<I1, I2, I3, O> {
        let input1 = input1.into_input();
        let input2 = input2.into_input();
        let input3 = input3.into_input();
        let id = self.next_id();
        let (id1, id2, id3) = (input1.id(), input2.id(), input3.id());
        self.add_edge(id1, id);
        self.add_edge(id2, id);
        self.add_edge(id3, id);
        let node = Rc::new(RefCell::new(map3::_Map3 {
            id,
            depth: min(min(input1.depth(), input2.depth()), input3.depth()) - 1,
            value: f(input1.observe(), input2.observe(), input3.observe()),
            input1: Some(input1),
            input2: Some(input2),
            input3: Some(input3),
            f: Some(Box::new(f)),
        }));
        self.nodes[id] = Some(node.clone());
        Map3 { node }
    }

    /// Creates a node that applies `f` to a homogeneous list of upstream values
    /// during stabilization.
    ///
    /// Panics if `inputs` is empty. The initial value is computed eagerly when the
    /// node is created. After that, `f` is called when any input has changed, and
    /// the result is only propagated if it differs from the previous output
    /// (cutoff optimization).
    pub fn mapn<T: Clone + 'static, O: PartialEq + 'static>(
        &mut self,
        inputs: Vec<Box<dyn Observable<T>>>,
        f: impl Fn(Vec<T>) -> O + 'static,
    ) -> MapN<T, O> {
        assert!(!inputs.is_empty(), "mapn requires at least one input");

        let id = self.next_id();
        for input in &inputs {
            self.add_edge(input.id(), id);
        }
        let depth = inputs
            .iter()
            .map(|input| input.depth())
            .min()
            .expect("mapn requires at least one input")
            - 1;
        let value = f(inputs.iter().map(|input| input.observe()).collect());
        let node = Rc::new(RefCell::new(mapn::_MapN {
            id,
            depth,
            value,
            inputs: Some(inputs),
            f: Some(Box::new(f)),
        }));
        self.nodes[id] = Some(node.clone());
        MapN { node }
    }

    /// Creates a node whose upstream dependency can change dynamically.
    ///
    /// The initial upstream node is selected eagerly when the bind is created.
    /// After that, `f` is called with the current value of `input` to select which
    /// node to read from. When `input` changes and `f` returns a different node,
    /// the graph is rewired and node depths are recalculated automatically. The
    /// bind only propagates downstream when its observed output actually changes.
    pub fn bind<I: 'static, O: Clone + PartialEq + 'static>(
        &mut self,
        input: impl IntoInput<I>,
        f: impl Fn(I) -> Box<dyn Observable<O>> + 'static,
    ) -> Bind1<I, O> {
        let input = input.into_input();
        let id = self.next_id();
        let input_id = input.id();
        let value = f(input.observe());
        let value_id = value.id();
        let depth = min(input.depth(), value.depth()) - 1;
        let node = Rc::new(RefCell::new(bind::_Bind1 {
            id,
            depth,
            current: value.observe(),
            value: Some(value),
            input: Some(input),
            f: Some(Box::new(f)),
        }));
        self.add_edge(input_id, id);
        self.add_edge(value_id, id);
        self.nodes[id] = Some(node.clone());
        Bind1 { node }
    }

    /// Registers a callback that runs after [`stabilize`](Incrementars::stabilize)
    /// whenever `input` changes.
    ///
    /// Pass a node handle directly, or use `.as_input()` when you specifically
    /// need a boxed input handle. Returns a watcher ID that
    /// can be removed later with [`unwatch`](Incrementars::unwatch).
    pub fn watch<T: Clone + 'static>(
        &mut self,
        input: impl IntoInput<T>,
        mut f: impl FnMut(T) + 'static,
    ) -> usize {
        let input = input.into_input();
        let id = self.hook_counter;
        self.hook_counter += 1;
        let node_id = input.id();
        self.hooks[node_id].push((
            id,
            Box::new(move || {
                f(input.observe());
            }),
        ));
        id
    }

    /// Removes a previously-registered watcher by ID.
    pub fn unwatch(&mut self, watcher_id: usize) -> bool {
        let mut removed = false;
        for hooks in &mut self.hooks {
            let before = hooks.len();
            hooks.retain(|(id, _)| *id != watcher_id);
            removed |= hooks.len() != before;
        }
        removed
    }

    /// Removes `node` and every downstream node that depends on it.
    ///
    /// Existing handles remain valid for direct observation, but the removed
    /// subgraph no longer participates in stabilization or future graph wiring.
    pub fn remove<T>(&mut self, node: &dyn Observable<T>) -> bool {
        let root_id = node.id();
        if self.nodes.get(root_id).and_then(Option::as_ref).is_none() {
            return false;
        }

        let mut to_remove = Bitmap::new(self.id_counter);
        let mut stack = vec![root_id];
        let mut removal_order = vec![];

        while let Some(node_id) = stack.pop() {
            if to_remove.contains(&node_id) {
                continue;
            }
            to_remove.insert(node_id);
            removal_order.push(node_id);
            stack.extend(self.dependencies[node_id].iter().copied());
        }

        for &node_id in &removal_order {
            let parents = self.reverse_dependencies[node_id].clone();
            for parent_id in parents {
                if !to_remove.contains(&parent_id) {
                    self.dependencies[parent_id].retain(|&child_id| child_id != node_id);
                }
            }
        }

        self.inputs.retain(|input| !to_remove.contains(&input.id()));

        for node_id in &removal_order {
            self.hooks[*node_id].clear();
            self.dependencies[*node_id].clear();
            self.reverse_dependencies[*node_id].clear();
        }

        for node in &mut self.reverse_dependencies {
            node.retain(|parent_id| !to_remove.contains(parent_id));
        }

        for node_id in removal_order {
            if let Some(node) = self.nodes[node_id].take() {
                node.borrow_mut().teardown();
            }
        }

        true
    }

    /// Propagates all pending changes through the graph.
    ///
    /// Dirty input nodes are discovered, then their dependents are processed in
    /// depth order (upstream before downstream). A node is skipped if its recomputed
    /// output equals its previous output (cutoff). After this call returns, all
    /// observable values reflect the latest inputs.
    pub fn stabilize(&mut self) {
        let mut queue = self
            .inputs
            .iter()
            .filter(|x| x.is_dirty())
            .map(|x| x.id())
            .map(|id| {
                let node = self.node(id);
                let node = node.deref().borrow();
                (node.depth(), node.id())
            })
            .collect::<BinaryHeap<(i32, usize)>>();

        let mut visited = Bitmap::new(self.id_counter);
        let mut changed = Bitmap::new(self.id_counter);
        let mut changed_nodes = vec![];

        while let Some((_depth, head_id)) = queue.pop() {
            let head = self.node(head_id);
            let callbacks = head.deref().borrow_mut().stabilize();

            for cb in callbacks {
                match cb {
                    StabilizationCallback::ValueChanged => {
                        if !changed.contains(&head_id) {
                            changed.insert(head_id);
                            changed_nodes.push(head_id);
                        }
                        for &child_id in &self.dependencies[head_id] {
                            if !visited.contains(&child_id) {
                                visited.insert(child_id);
                                let child = self.node(child_id);
                                let depth = child.deref().borrow().depth();
                                queue.push((depth, child_id));
                            }
                        }
                    }
                    StabilizationCallback::DependenciesUpdated { from, to } => {
                        // Remove old dependency edges
                        for from_id in &from {
                            self.dependencies[*from_id].retain(|&x| x != head_id);
                            self.reverse_dependencies[head_id].retain(|&x| x != *from_id);
                        }
                        // Add new dependency edges
                        for &to_id in &to {
                            self.add_edge(to_id, head_id);
                        }

                        // Adjust depths for head_id and all its descendants.
                        // Runs both up and down since the new target may be at a
                        // different depth than the old one.
                        let mut adjust_queue = vec![head_id];
                        while let Some(node_id) = adjust_queue.pop() {
                            let min_parent_depth = self.reverse_dependencies[node_id]
                                .iter()
                                .filter_map(|&pid| {
                                    self.nodes[pid].as_ref().map(|node| node.borrow().depth())
                                })
                                .min();

                            if let Some(parent_depth) = min_parent_depth {
                                let new_depth = parent_depth - 1;
                                let node = self.node(node_id);
                                let old_depth = node.borrow().depth();
                                if new_depth != old_depth {
                                    node.borrow_mut().adjust_depth(new_depth);
                                    adjust_queue.extend(self.dependencies[node_id].iter().copied());
                                }
                            }
                        }
                    }
                }
            }
        }

        for node_id in changed_nodes {
            for (_, hook) in self.hooks[node_id].iter_mut() {
                hook();
            }
        }
    }
}

impl Default for Incrementars {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Incrementars {
    fn drop(&mut self) {
        for node in self.nodes.iter_mut().filter_map(Option::as_mut) {
            node.borrow_mut().teardown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn var_instantiation() {
        let mut dag = Incrementars::new();
        let var = dag.var(0);
        var.set(10);
        assert_eq!(var.observe(), 10);
    }

    #[test]
    fn map_instantiation() {
        let mut dag = Incrementars::new();
        let var = dag.var(0);
        let map = dag.map(var, |x| x + 1);
        assert_eq!(map.observe(), 1);
    }

    #[test]
    fn bifurcate() {
        let mut dag = Incrementars::new();
        let var = dag.var(0);
        let map = dag.map(var.as_input(), |x| x + 1);
        let map2 = dag.map(var.as_input(), |x| x + 1);
        assert_eq!(map.observe(), 1);
        assert_eq!(map2.observe(), 1);

        var.set(10);
        assert_eq!(map.observe(), 1);
        dag.stabilize();
        assert_eq!(map.observe(), 11);
        assert_eq!(map2.observe(), 11);
    }

    #[test]
    fn test_map2() {
        let mut dag = Incrementars::new();
        let var1 = dag.var(50);
        let var2 = dag.var(" dollars");
        let map2 = dag.map2(var1.as_input(), var2.as_input(), |x, y| x.to_string() + y);
        assert_eq!(map2.observe(), "50 dollars");
    }

    #[test]
    fn test_combinatoric() {
        let mut dag = Incrementars::new();
        let var1 = dag.var(50);
        let plus_one = |x| x + 1;
        let var21 = dag.map(var1.as_input(), plus_one);
        let var22 = dag.map(var21.as_input(), plus_one);
        let var23 = dag.map(var22.as_input(), plus_one);
        let var31 = dag.map(var1.as_input(), plus_one);
        let rejoin = dag.map2(var31.as_input(), var23.as_input(), |x, y| x + y);

        var1.set(10);
        dag.stabilize();
        assert_eq!(rejoin.observe(), 24);
    }

    #[test]
    fn test_map3() {
        let mut dag = Incrementars::new();
        let left = dag.var(2);
        let middle = dag.var(3);
        let right = dag.var(4);
        let product = dag.map3(
            left.as_input(),
            middle.as_input(),
            right.as_input(),
            |x, y, z| x * y * z,
        );
        assert_eq!(product.observe(), 24);

        middle.set(5);
        dag.stabilize();
        assert_eq!(product.observe(), 40);
    }

    #[test]
    fn test_mapn() {
        let mut dag = Incrementars::new();
        let a = dag.var(2);
        let b = dag.var(3);
        let c = dag.var(4);
        let total = dag.mapn(vec![a.as_input(), b.as_input(), c.as_input()], |values| {
            values.into_iter().sum::<i32>()
        });
        assert_eq!(total.observe(), 9);

        b.set(10);
        dag.stabilize();
        assert_eq!(total.observe(), 16);
    }

    #[test]
    fn test_observe_ref_non_clone_output() {
        let mut dag = Incrementars::new();
        let x = dag.var(3);
        let text = dag.map(x.clone(), |value| format!("value={value}").into_bytes());

        assert_eq!(text.observe_ref().as_slice(), b"value=3");

        x.set(8);
        dag.stabilize();
        assert_eq!(text.observe_ref().as_slice(), b"value=8");
    }

    #[test]
    fn test_bind() {
        let mut dag = Incrementars::new();
        let left = dag.var(1);
        let right = dag.var(2);
        let left_id = traits::Observable::id(&left);
        let right_id = traits::Observable::id(&right);

        #[derive(Debug, Clone, Copy)]
        enum Side {
            Left,
            Right,
        }

        let picker = dag.var(Side::Left);

        fn pick(
            left: Box<Var<i32>>,
            right: Box<Var<i32>>,
        ) -> impl Fn(Side) -> Box<dyn Observable<i32>> {
            move |side| match side {
                Side::Left => left.clone(),
                Side::Right => right.clone(),
            }
        }

        let binder = dag.bind(picker.as_input(), pick(left.as_input(), right.as_input()));
        let binder_id = binder.id();

        assert_eq!(dag.dependencies[left_id], vec![binder.id()]);
        assert!(dag.dependencies[right_id].is_empty());
        assert_eq!(binder.observe(), 1);
        picker.set(Side::Right);
        dag.stabilize();
        assert_eq!(binder.observe(), 2);
        assert_eq!(dag.dependencies[right_id], vec![binder_id]);
        assert!(dag.dependencies[left_id].is_empty());
    }

    #[test]
    fn test_bind_adjust_depth_propagation() {
        let mut dag = Incrementars::new();
        let left_root = dag.var(1);
        let right_root = dag.var(-1);
        let left_map = dag.map(left_root.as_input(), |x| x * 2);

        #[derive(Debug, Clone, Copy)]
        enum Side {
            Left,
            Right,
        }

        let picker = dag.var(Side::Right);

        fn pick(
            left: Box<Map1<i32, i32>>,
            right: Box<Var<i32>>,
        ) -> impl Fn(Side) -> Box<dyn Observable<i32>> {
            move |side| match side {
                Side::Left => left.clone(),
                Side::Right => right.clone(),
            }
        }

        let binder = dag.bind(
            picker.as_input(),
            pick(left_map.as_input(), right_root.as_input()),
        );

        let map_after_bind = dag.map(binder.as_input(), |n| n * 10);
        let binder_old_depth = binder.depth();
        let mabind_old_depth = map_after_bind.depth();

        picker.set(Side::Left);
        dag.stabilize();
        let binder_new_depth = binder.depth();
        let mabind_new_depth = map_after_bind.depth();

        assert_eq!(binder_new_depth, binder_old_depth - 1);
        assert_eq!(mabind_new_depth, mabind_old_depth - 1);
    }

    #[test]
    fn test_bind_propagates_inner_value_changes() {
        let mut dag = Incrementars::new();
        let selected = dag.var(10);
        let chooser = dag.var(());
        let selected_for_bind = selected.clone();
        let binder = dag.bind(chooser.as_input(), move |_| selected_for_bind.as_input());
        let downstream = dag.map(binder.as_input(), |x| x + 1);

        assert_eq!(downstream.observe(), 11);

        selected.set(41);
        dag.stabilize();
        assert_eq!(binder.observe(), 41);
        assert_eq!(downstream.observe(), 42);
    }

    #[test]
    fn test_bind_same_node_same_value_does_not_recompute_downstream() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut dag = Incrementars::new();
        let chooser = dag.var(0);
        let selected = dag.var(10);
        let selected_for_bind = selected.clone();
        let binder = dag.bind(chooser.clone(), move |_| selected_for_bind.as_input());

        let seen = Arc::clone(&counter);
        dag.map(binder.as_input(), move |value| {
            seen.fetch_add(1, Ordering::SeqCst);
            value + 1
        });

        assert_eq!(counter.load(Ordering::SeqCst), 1);

        chooser.set(1);
        dag.stabilize();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_bind_rewire_same_value_does_not_recompute_downstream() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut dag = Incrementars::new();
        let left = dag.var(10);
        let right = dag.var(10);

        #[derive(Debug, Clone, Copy)]
        enum Side {
            Left,
            Right,
        }

        let chooser = dag.var(Side::Left);

        fn pick(
            left: Box<Var<i32>>,
            right: Box<Var<i32>>,
        ) -> impl Fn(Side) -> Box<dyn Observable<i32>> {
            move |side| match side {
                Side::Left => left.clone(),
                Side::Right => right.clone(),
            }
        }

        let binder = dag.bind(chooser.clone(), pick(left.as_input(), right.as_input()));
        let seen = Arc::clone(&counter);
        dag.map(binder.as_input(), move |value| {
            seen.fetch_add(1, Ordering::SeqCst);
            value + 1
        });

        assert_eq!(counter.load(Ordering::SeqCst), 1);

        chooser.set(Side::Right);
        dag.stabilize();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(binder.observe(), 10);
    }

    #[test]
    fn test_real_life() {
        let mut dag = Incrementars::new();
        let length = dag.var(2.0);
        let area = dag.map(length.as_input(), |x| x * x);

        assert_eq!(area.observe(), 4.0);
        length.set(3.0);
        assert_eq!(area.observe(), 4.0);

        dag.stabilize();
        assert_eq!(area.observe(), 9.0);

        let height = dag.var(5.0);
        let volume = dag.map2(area.as_input(), height.as_input(), |x, y| x * y);

        assert_eq!(volume.observe(), 45.0);

        height.set(10.0);
        dag.stabilize();
        assert_eq!(volume.observe(), 90.0);
    }

    #[test]
    fn test_combinatorial_only_fire_once_at_combine() {
        let counter = Arc::new(AtomicUsize::new(0));

        let mut dag = Incrementars::new();
        let var1 = dag.var(1);
        let plus_one = |x| x + 1;
        let left1 = dag.map(var1.as_input(), plus_one);
        let left2 = dag.map(left1.as_input(), plus_one);
        let left3 = dag.map(left2.as_input(), plus_one);
        let right = dag.map(var1.as_input(), plus_one);

        let c = Arc::clone(&counter);
        dag.map2(left3.as_input(), right.as_input(), move |_, _| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_map2_same_input_twice() {
        // Regression: passing the same node as both inputs to map2 should not
        // create duplicate edges or cause incorrect behaviour.
        let mut dag = Incrementars::new();
        let x = dag.var(3);
        let squared = dag.map2(x.as_input(), x.as_input(), |a, b| a * b);
        assert_eq!(squared.observe(), 9);

        x.set(4);
        dag.stabilize();
        assert_eq!(squared.observe(), 16);
    }

    #[test]
    fn test_watch_and_unwatch() {
        let mut dag = Incrementars::new();
        let x = dag.var(2);
        let doubled = dag.map(x.as_input(), |value| value * 2);
        let seen = Arc::new(AtomicUsize::new(0));

        let seen_clone = Arc::clone(&seen);
        let watch_id = dag.watch(doubled.as_input(), move |value| {
            seen_clone.store(value as usize, Ordering::SeqCst);
        });

        x.set(5);
        dag.stabilize();
        assert_eq!(seen.load(Ordering::SeqCst), 10);

        assert!(dag.unwatch(watch_id));
        x.set(7);
        dag.stabilize();
        assert_eq!(seen.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_remove_subgraph() {
        let mut dag = Incrementars::new();
        let x = dag.var(1);
        let y = dag.var(10);
        let left = dag.map(x.as_input(), |value| value + 1);
        let right = dag.map(left.as_input(), |value| value * 2);
        let unaffected = dag.map(y.as_input(), |value| value + 5);

        assert!(dag.remove(&left));
        assert!(dag.nodes[left.id()].is_none());
        assert!(dag.nodes[right.id()].is_none());
        assert!(dag.nodes[traits::Observable::id(&x)].is_some());
        assert!(dag.nodes[traits::Observable::id(&y)].is_some());

        y.set(20);
        dag.stabilize();
        assert_eq!(unaffected.observe(), 25);
    }
}
