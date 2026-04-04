use std::cmp::max;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Reverse;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use bitmap::Bitmap;
use traits::{Cutoff, StabilizationResult};

mod bind;
mod bitmap;
mod map;
mod map2;
mod map3;
mod mapn;
mod traits;
mod var;

pub use self::{
    traits::{Bind, IntoInput, Map1, Map2, Map3, MapN, Observable, Signal, ValueState},
    var::Var,
};

// Vars are assigned height 0. Derived nodes add 1 per level, so the stabilization
// queue (a min-heap on height) always processes inputs before their dependents.
const VAR_HEIGHT: i32 = 0;

/// A bucket queue (dial's algorithm) for the stabilization recompute queue.
///
/// O(1) push (indexed by height), O(max_height) full drain — better than the
/// O(log n) per-operation cost of `BinaryHeap` for graphs with bounded heights.
struct BucketQueue {
    buckets: Vec<Vec<usize>>,
    min_height: usize,
}

impl BucketQueue {
    fn new() -> Self {
        BucketQueue {
            buckets: Vec::new(),
            min_height: 0,
        }
    }

    fn push(&mut self, height: usize, id: usize) {
        if height >= self.buckets.len() {
            self.buckets.resize_with(height + 1, Vec::new);
        }
        self.buckets[height].push(id);
        if height < self.min_height {
            self.min_height = height;
        }
    }

    fn pop(&mut self) -> Option<usize> {
        while self.min_height < self.buckets.len() {
            if let Some(id) = self.buckets[self.min_height].pop() {
                return Some(id);
            }
            self.min_height += 1;
        }
        None
    }
}

pub struct DirtyInput {
    id: usize,
    dirty: Rc<Cell<bool>>,
}

/// The incremental computation graph.
///
/// Create input nodes with [`var`](Graph::var), wire them together with
/// [`map`](Graph::map), [`map2`](Graph::map2),
/// [`map3`](Graph::map3), [`mapn`](Graph::mapn), and
/// [`bind`](Graph::bind), then call [`stabilize`](Graph::stabilize)
/// to propagate pending changes through the graph.
///
/// # Example
/// ```
/// use incrementars::prelude::*;
/// let mut dag = Graph::new();
/// let x = dag.var(2);
/// let y = dag.map(&x, |v| v * v);
/// assert_eq!(y.observe(), 4);
/// x.set(3);
/// dag.stabilize();
/// assert_eq!(y.observe(), 9);
/// ```
pub struct Graph {
    // node id → node handle. IDs are assigned sequentially from 0 by next_id(),
    // and remain stable even after removal.
    nodes: Vec<Option<Box<dyn traits::InternalNode>>>,
    id_counter: usize,

    inputs: Vec<DirtyInput>,
    // parent_id → [child_ids]: which nodes depend on a given node
    pub(crate) dependencies: Vec<Vec<usize>>,
    // child_id → [parent_ids]: which nodes a given node depends on
    reverse_dependencies: Vec<Vec<usize>>,
    // set of (parent_id, child_id) edges for O(1) deduplication in add_edge
    edge_set: HashSet<(usize, usize)>,
    hooks: Vec<Vec<(usize, Box<dyn FnMut()>)>>,
    hook_counter: usize,
    // hook_id → node_id for O(1) unwatch
    hook_index: HashMap<usize, usize>,
}

impl Graph {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            id_counter: 0,
            inputs: vec![],
            dependencies: vec![],
            reverse_dependencies: vec![],
            edge_set: HashSet::new(),
            hooks: vec![],
            hook_counter: 0,
            hook_index: HashMap::new(),
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

    fn node(&self, id: usize) -> &dyn traits::InternalNode {
        self.nodes[id]
            .as_deref()
            .expect("node id missing from graph")
    }

    fn node_mut(&mut self, id: usize) -> &mut dyn traits::InternalNode {
        self.nodes[id]
            .as_deref_mut()
            .expect("node id missing from graph")
    }

    /// Adds a directed edge parent → child to both dependency maps.
    /// Silently deduplicates: calling with the same pair twice has no effect.
    fn add_edge(&mut self, parent_id: usize, child_id: usize) {
        if self.edge_set.insert((parent_id, child_id)) {
            self.dependencies[parent_id].push(child_id);
            self.reverse_dependencies[child_id].push(parent_id);
        }
    }

    /// Creates an input node holding `value`.
    ///
    /// Use [`Var::set`] to update the value. Changes are not visible to downstream
    /// nodes until the next call to [`stabilize`](Graph::stabilize).
    pub fn var<T: Clone + 'static>(&mut self, value: T) -> Var<T> {
        let id = self.next_id();
        let state = Rc::new(RefCell::new(traits::ValueState::new(id, VAR_HEIGHT, value)));
        let dirty = Rc::new(Cell::new(false));
        self.nodes[id] = Some(Box::new(var::VarNode::new(state.clone(), dirty.clone())));
        self.inputs.push(DirtyInput {
            id,
            dirty: dirty.clone(),
        });
        Var { state, dirty }
    }

    /// Creates a node that applies `f` to the output of `input` during stabilization.
    ///
    /// The initial value is computed eagerly when the node is created. After that,
    /// `f` is only called when `input` has changed since the last stabilization, and
    /// its result is only propagated downstream if it differs from the previous
    /// output (cutoff optimization, requires `O: PartialEq`).
    pub fn map<I: Clone + 'static, O: PartialEq + 'static>(
        &mut self,
        input: impl IntoInput<I>,
        f: impl Fn(I) -> O + 'static,
    ) -> Map1<I, O> {
        let input = input.into_input();
        let id = self.next_id();
        let input_id = input.id();
        self.add_edge(input_id, id);
        let output = Signal::new(id, input.depth() + 1, f(input.observe()));
        let node = Box::new(map::MapNode1 {
            output: output.clone(),
            input: Some(input),
            f: Some(Box::new(f)),
            cutoff: Some(Cutoff::Direct(<O as PartialEq>::eq)),
        });
        self.nodes[id] = Some(node);
        Map1 {
            inner: output,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Like [`map`](Graph::map) but with a custom cutoff predicate instead of `PartialEq`.
    ///
    /// `cutoff(old, new)` returning `true` suppresses downstream propagation.
    /// Pass `|_, _| false` to always propagate (no cutoff). Does not require `O: PartialEq`.
    pub fn map_with_cutoff<I: Clone + 'static, O: 'static>(
        &mut self,
        input: impl IntoInput<I>,
        f: impl Fn(I) -> O + 'static,
        cutoff: impl Fn(&O, &O) -> bool + 'static,
    ) -> Map1<I, O> {
        let input = input.into_input();
        let id = self.next_id();
        let input_id = input.id();
        self.add_edge(input_id, id);
        let output = Signal::new(id, input.depth() + 1, f(input.observe()));
        let node = Box::new(map::MapNode1 {
            output: output.clone(),
            input: Some(input),
            f: Some(Box::new(f)),
            cutoff: Some(Cutoff::Custom(Box::new(cutoff))),
        });
        self.nodes[id] = Some(node);
        Map1 {
            inner: output,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a node that applies `f` to the outputs of `input1` and `input2`
    /// during stabilization.
    ///
    /// The initial value is computed eagerly when the node is created. After that,
    /// `f` is called when either input has changed, and the result is only
    /// propagated if it differs from the previous output (cutoff optimization).
    pub fn map2<I1: Clone + 'static, I2: Clone + 'static, O: PartialEq + 'static>(
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
        let output = Signal::new(
            id,
            max(input1.depth(), input2.depth()) + 1,
            f(input1.observe(), input2.observe()),
        );
        let node = Box::new(map2::MapNode2 {
            output: output.clone(),
            input1: Some(input1),
            input2: Some(input2),
            f: Some(Box::new(f)),
            cutoff: Some(Cutoff::Direct(<O as PartialEq>::eq)),
        });
        self.nodes[id] = Some(node);
        Map2 {
            inner: output,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a node that applies `f` to the outputs of `input1`, `input2`,
    /// and `input3` during stabilization.
    ///
    /// The initial value is computed eagerly when the node is created. After that,
    /// `f` is called when any input has changed, and the result is only propagated
    /// if it differs from the previous output (cutoff optimization).
    pub fn map3<
        I1: Clone + 'static,
        I2: Clone + 'static,
        I3: Clone + 'static,
        O: PartialEq + 'static,
    >(
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
        let output = Signal::new(
            id,
            max(max(input1.depth(), input2.depth()), input3.depth()) + 1,
            f(input1.observe(), input2.observe(), input3.observe()),
        );
        let node = Box::new(map3::MapNode3 {
            output: output.clone(),
            input1: Some(input1),
            input2: Some(input2),
            input3: Some(input3),
            f: Some(Box::new(f)),
            cutoff: Some(Cutoff::Direct(<O as PartialEq>::eq)),
        });
        self.nodes[id] = Some(node);
        Map3 {
            inner: output,
            _phantom: std::marker::PhantomData,
        }
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
        inputs: impl IntoIterator<Item = impl IntoInput<T>>,
        f: impl Fn(Vec<T>) -> O + 'static,
    ) -> MapN<T, O> {
        let inputs = inputs
            .into_iter()
            .map(IntoInput::into_input)
            .collect::<Vec<_>>();
        assert!(!inputs.is_empty(), "mapn requires at least one input");

        let id = self.next_id();
        for input in &inputs {
            self.add_edge(input.id(), id);
        }
        let depth = inputs
            .iter()
            .map(|input| input.depth())
            .max()
            .expect("mapn requires at least one input")
            + 1;
        let output = Signal::new(
            id,
            depth,
            f(inputs.iter().map(|input| input.observe()).collect()),
        );
        let node = Box::new(mapn::MapNodeN {
            output: output.clone(),
            inputs: Some(inputs),
            f: Some(Box::new(f)),
            cutoff: Some(Cutoff::Direct(<O as PartialEq>::eq)),
        });
        self.nodes[id] = Some(node);
        MapN {
            inner: output,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a node whose upstream dependency can change dynamically.
    ///
    /// The initial upstream node is selected eagerly when the bind is created.
    /// After that, `f` is called with the current value of `input` to select which
    /// node to read from. When `input` changes and `f` returns a different node,
    /// the graph is rewired and node depths are recalculated automatically. The
    /// bind only propagates downstream when its observed output actually changes.
    pub fn bind<I: Clone + 'static, O: Clone + PartialEq + 'static>(
        &mut self,
        input: impl IntoInput<I>,
        f: impl Fn(I) -> Signal<O> + 'static,
    ) -> Bind<I, O> {
        let input = input.into_input();
        let id = self.next_id();
        let input_id = input.id();
        let value = f(input.observe());
        let value_id = value.id();
        let depth = max(input.depth(), value.depth()) + 1;
        let output = Signal::new(id, depth, value.observe());
        let node = Box::new(bind::BindNode {
            output: output.clone(),
            value: Some(value),
            input: Some(input),
            f: Some(Box::new(f)),
        });
        self.add_edge(input_id, id);
        self.add_edge(value_id, id);
        self.nodes[id] = Some(node);
        Bind {
            inner: output,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Registers a callback that runs after [`stabilize`](Graph::stabilize)
    /// whenever `input` changes.
    ///
    /// Pass a node handle directly, including `&node`. Returns a watcher ID that
    /// can be removed later with [`unwatch`](Graph::unwatch).
    pub fn watch<T: Clone + 'static>(
        &mut self,
        input: impl IntoInput<T>,
        mut f: impl FnMut(T) + 'static,
    ) -> usize {
        let input = input.into_input();
        let hook_id = self.hook_counter;
        self.hook_counter += 1;
        let node_id = input.id();
        self.hooks[node_id].push((
            hook_id,
            Box::new(move || {
                f(input.observe());
            }),
        ));
        self.hook_index.insert(hook_id, node_id);
        hook_id
    }

    /// Removes a previously-registered watcher by ID.
    pub fn unwatch(&mut self, watcher_id: usize) -> bool {
        if let Some(&node_id) = self.hook_index.get(&watcher_id) {
            self.hooks[node_id].retain(|(id, _)| *id != watcher_id);
            self.hook_index.remove(&watcher_id);
            true
        } else {
            false
        }
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
                    self.edge_set.remove(&(parent_id, node_id));
                }
            }
        }

        self.inputs.retain(|input| !to_remove.contains(&input.id));

        for &node_id in &removal_order {
            for (hook_id, _) in self.hooks[node_id].drain(..) {
                self.hook_index.remove(&hook_id);
            }
            for &child_id in &self.dependencies[node_id] {
                self.edge_set.remove(&(node_id, child_id));
            }
            for &parent_id in &self.reverse_dependencies[node_id] {
                self.edge_set.remove(&(parent_id, node_id));
            }
            self.dependencies[node_id].clear();
            self.reverse_dependencies[node_id].clear();
        }

        for node in &mut self.reverse_dependencies {
            node.retain(|parent_id| !to_remove.contains(parent_id));
        }

        for node_id in removal_order {
            if let Some(node) = self.nodes[node_id].take() {
                let mut node = node;
                node.teardown();
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
        let mut queue = BucketQueue::new();
        for input in self.inputs.iter().filter(|input| input.dirty.get()) {
            let height = self.node(input.id).depth() as usize;
            queue.push(height, input.id);
        }

        let mut visited = Bitmap::new(self.id_counter);
        let mut changed = Bitmap::new(self.id_counter);
        let mut changed_nodes = vec![];

        while let Some(head_id) = queue.pop() {
            let callbacks = {
                let head = self.node_mut(head_id);
                head.stabilize()
            };

            match callbacks {
                StabilizationResult::Unchanged => {}
                StabilizationResult::Changed => {
                    if !changed.contains(&head_id) {
                        changed.insert(head_id);
                        changed_nodes.push(head_id);
                    }
                    for &child_id in &self.dependencies[head_id] {
                        if !visited.contains(&child_id) {
                            visited.insert(child_id);
                            let height = self.node(child_id).depth() as usize;
                            queue.push(height, child_id);
                        }
                    }
                }
                StabilizationResult::Rebound {
                    from,
                    to,
                    value_changed,
                } => {
                    self.dependencies[from].retain(|&x| x != head_id);
                    self.reverse_dependencies[head_id].retain(|&x| x != from);
                    self.add_edge(to, head_id);

                    // Use a min-heap ordered by current height so parents are always
                    // settled before their children (topological order). A DFS stack
                    // could visit a shared descendant before all of its parents have
                    // been updated, producing a stale max() and wrong height on diamond
                    // graphs.
                    let mut adjust_heap: BinaryHeap<(Reverse<i32>, usize)> = BinaryHeap::new();
                    adjust_heap.push((Reverse(self.node(head_id).depth()), head_id));
                    // Height upper bound for a DAG: at most id_counter levels.
                    // A cycle via bind would push heights toward infinity, so any
                    // height exceeding this bound means a cycle was introduced.
                    let max_valid_height = self.id_counter as i32;
                    while let Some((_, node_id)) = adjust_heap.pop() {
                        let max_parent_height = self.reverse_dependencies[node_id]
                            .iter()
                            .filter_map(|&pid| self.nodes[pid].as_ref().map(|node| node.depth()))
                            .max();

                        if let Some(parent_height) = max_parent_height {
                            let new_height = parent_height + 1;
                            assert!(
                                new_height <= max_valid_height,
                                "incrementars: cycle detected during height adjustment (node {node_id})"
                            );
                            if new_height != self.node(node_id).depth() {
                                self.node_mut(node_id).adjust_depth(new_height);
                                for &child_id in &self.dependencies[node_id] {
                                    adjust_heap.push((Reverse(self.node(child_id).depth()), child_id));
                                }
                            }
                        }
                    }

                    if value_changed && !changed.contains(&head_id) {
                        changed.insert(head_id);
                        changed_nodes.push(head_id);
                    }
                    if value_changed {
                        for &child_id in &self.dependencies[head_id] {
                            if !visited.contains(&child_id) {
                                visited.insert(child_id);
                                let height = self.node(child_id).depth() as usize;
                                queue.push(height, child_id);
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

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Graph {
    fn drop(&mut self) {
        for node in self.nodes.iter_mut().filter_map(Option::as_mut) {
            node.teardown();
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
        let mut dag = Graph::new();
        let var = dag.var(0);
        var.set(10);
        assert_eq!(var.observe(), 10);
    }

    #[test]
    fn map_instantiation() {
        let mut dag = Graph::new();
        let var = dag.var(0);
        let map = dag.map(var, |x| x + 1);
        assert_eq!(map.observe(), 1);
    }

    #[test]
    fn bifurcate() {
        let mut dag = Graph::new();
        let var = dag.var(0);
        let map = dag.map(&var, |x| x + 1);
        let map2 = dag.map(&var, |x| x + 1);
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
        let mut dag = Graph::new();
        let var1 = dag.var(50);
        let var2 = dag.var(" dollars");
        let map2 = dag.map2(&var1, &var2, |x, y| x.to_string() + y);
        assert_eq!(map2.observe(), "50 dollars");
    }

    #[test]
    fn test_combinatoric() {
        let mut dag = Graph::new();
        let var1 = dag.var(50);
        let plus_one = |x| x + 1;
        let var21 = dag.map(&var1, plus_one);
        let var22 = dag.map(&var21, plus_one);
        let var23 = dag.map(&var22, plus_one);
        let var31 = dag.map(&var1, plus_one);
        let rejoin = dag.map2(&var31, &var23, |x, y| x + y);

        var1.set(10);
        dag.stabilize();
        assert_eq!(rejoin.observe(), 24);
    }

    #[test]
    fn test_map3() {
        let mut dag = Graph::new();
        let left = dag.var(2);
        let middle = dag.var(3);
        let right = dag.var(4);
        let product = dag.map3(&left, &middle, &right, |x, y, z| x * y * z);
        assert_eq!(product.observe(), 24);

        middle.set(5);
        dag.stabilize();
        assert_eq!(product.observe(), 40);
    }

    #[test]
    fn test_mapn() {
        let mut dag = Graph::new();
        let a = dag.var(2);
        let b = dag.var(3);
        let c = dag.var(4);
        let total = dag.mapn([&a, &b, &c], |values| values.into_iter().sum::<i32>());
        assert_eq!(total.observe(), 9);

        b.set(10);
        dag.stabilize();
        assert_eq!(total.observe(), 16);
    }

    #[test]
    fn test_observe_ref_non_clone_output() {
        let mut dag = Graph::new();
        let x = dag.var(3);
        let text = dag.map(x.clone(), |value| format!("value={value}").into_bytes());

        assert_eq!(text.observe_ref().as_slice(), b"value=3");

        x.set(8);
        dag.stabilize();
        assert_eq!(text.observe_ref().as_slice(), b"value=8");
    }

    #[test]
    fn test_bind() {
        let mut dag = Graph::new();
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

        fn pick(left: Var<i32>, right: Var<i32>) -> impl Fn(Side) -> Signal<i32> {
            move |side| match side {
                Side::Left => left.clone().into_input(),
                Side::Right => right.clone().into_input(),
            }
        }

        let binder = dag.bind(&picker, pick(left.clone(), right.clone()));
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
        let mut dag = Graph::new();
        let left_root = dag.var(1);
        let right_root = dag.var(-1);
        let left_map = dag.map(&left_root, |x| x * 2);

        #[derive(Debug, Clone, Copy)]
        enum Side {
            Left,
            Right,
        }

        let picker = dag.var(Side::Right);

        fn pick(left: Map1<i32, i32>, right: Var<i32>) -> impl Fn(Side) -> Signal<i32> {
            move |side| match side {
                Side::Left => left.clone().into_input(),
                Side::Right => right.clone().into_input(),
            }
        }

        let binder = dag.bind(&picker, pick(left_map.clone(), right_root.clone()));

        let map_after_bind = dag.map(&binder, |n| n * 10);
        let binder_old_depth = binder.depth();
        let mabind_old_depth = map_after_bind.depth();

        picker.set(Side::Left);
        dag.stabilize();
        let binder_new_depth = binder.depth();
        let mabind_new_depth = map_after_bind.depth();

        assert_eq!(binder_new_depth, binder_old_depth + 1);
        assert_eq!(mabind_new_depth, mabind_old_depth + 1);
    }

    #[test]
    fn test_bind_propagates_inner_value_changes() {
        let mut dag = Graph::new();
        let selected = dag.var(10);
        let chooser = dag.var(());
        let selected_for_bind = selected.clone();
        let binder = dag.bind(&chooser, move |_| selected_for_bind.clone().into_input());
        let downstream = dag.map(&binder, |x| x + 1);

        assert_eq!(downstream.observe(), 11);

        selected.set(41);
        dag.stabilize();
        assert_eq!(binder.observe(), 41);
        assert_eq!(downstream.observe(), 42);
    }

    #[test]
    fn test_bind_same_node_same_value_does_not_recompute_downstream() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut dag = Graph::new();
        let chooser = dag.var(0);
        let selected = dag.var(10);
        let selected_for_bind = selected.clone();
        let binder = dag.bind(&chooser, move |_| selected_for_bind.clone().into_input());

        let seen = Arc::clone(&counter);
        dag.map(&binder, move |value| {
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
        let mut dag = Graph::new();
        let left = dag.var(10);
        let right = dag.var(10);

        #[derive(Debug, Clone, Copy)]
        enum Side {
            Left,
            Right,
        }

        let chooser = dag.var(Side::Left);

        fn pick(left: Var<i32>, right: Var<i32>) -> impl Fn(Side) -> Signal<i32> {
            move |side| match side {
                Side::Left => left.clone().into_input(),
                Side::Right => right.clone().into_input(),
            }
        }

        let binder = dag.bind(&chooser, pick(left.clone(), right.clone()));
        let seen = Arc::clone(&counter);
        dag.map(&binder, move |value| {
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
        let mut dag = Graph::new();
        let length = dag.var(2.0);
        let area = dag.map(&length, |x| x * x);

        assert_eq!(area.observe(), 4.0);
        length.set(3.0);
        assert_eq!(area.observe(), 4.0);

        dag.stabilize();
        assert_eq!(area.observe(), 9.0);

        let height = dag.var(5.0);
        let volume = dag.map2(&area, &height, |x, y| x * y);

        assert_eq!(volume.observe(), 45.0);

        height.set(10.0);
        dag.stabilize();
        assert_eq!(volume.observe(), 90.0);
    }

    #[test]
    fn test_combinatorial_only_fire_once_at_combine() {
        let counter = Arc::new(AtomicUsize::new(0));

        let mut dag = Graph::new();
        let var1 = dag.var(1);
        let plus_one = |x| x + 1;
        let left1 = dag.map(&var1, plus_one);
        let left2 = dag.map(&left1, plus_one);
        let left3 = dag.map(&left2, plus_one);
        let right = dag.map(&var1, plus_one);

        let c = Arc::clone(&counter);
        dag.map2(&left3, &right, move |_, _| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_map2_same_input_twice() {
        // Regression: passing the same node as both inputs to map2 should not
        // create duplicate edges or cause incorrect behaviour.
        let mut dag = Graph::new();
        let x = dag.var(3);
        let squared = dag.map2(&x, &x, |a, b| a * b);
        assert_eq!(squared.observe(), 9);

        x.set(4);
        dag.stabilize();
        assert_eq!(squared.observe(), 16);
    }

    #[test]
    fn test_watch_and_unwatch() {
        let mut dag = Graph::new();
        let x = dag.var(2);
        let doubled = dag.map(&x, |value| value * 2);
        let seen = Arc::new(AtomicUsize::new(0));

        let seen_clone = Arc::clone(&seen);
        let watch_id = dag.watch(&doubled, move |value| {
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
    fn test_set_if_changed_does_not_propagate() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut dag = Graph::new();
        let x = dag.var(5);
        let c = Arc::clone(&counter);
        dag.map(&x, move |v| {
            c.fetch_add(1, Ordering::SeqCst);
            v + 1
        });
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // set_if_changed with same value — should not trigger recomputation
        x.set_if_changed(5);
        dag.stabilize();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // set_if_changed with new value — should trigger recomputation
        x.set_if_changed(10);
        dag.stabilize();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_map_with_cutoff() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut dag = Graph::new();
        let x = dag.var(1.0f64);
        // Cutoff: suppress downstream propagation when output changes by less than 0.5
        let out = dag.map_with_cutoff(&x, |v| v * 2.0, |old, new| (old - new).abs() < 0.5);

        // Wire a downstream node that counts recomputations
        let c = Arc::clone(&counter);
        let downstream = dag.map(&out, move |v| {
            c.fetch_add(1, Ordering::SeqCst);
            v + 1.0
        });
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(downstream.observe(), 3.0);

        // x=1.1 → out would be 2.2; |2.0 - 2.2| = 0.2 < 0.5, cutoff fires
        // downstream should NOT recompute
        x.set(1.1);
        dag.stabilize();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        // out retains old value because cutoff suppressed the update
        assert_eq!(out.observe(), 2.0);

        // x=5.0 → out would be 10.0; |2.0 - 10.0| = 8.0 >= 0.5, cutoff does not fire
        // downstream SHOULD recompute
        x.set(5.0);
        dag.stabilize();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(out.observe(), 10.0);
        assert_eq!(downstream.observe(), 11.0);
    }

    #[test]
    fn test_remove_subgraph() {
        let mut dag = Graph::new();
        let x = dag.var(1);
        let y = dag.var(10);
        let left = dag.map(&x, |value| value + 1);
        let right = dag.map(&left, |value| value * 2);
        let unaffected = dag.map(&y, |value| value + 5);

        assert!(dag.remove(&left));
        assert!(dag.nodes[left.id()].is_none());
        assert!(dag.nodes[right.id()].is_none());
        assert!(dag.nodes[traits::Observable::id(&x)].is_some());
        assert!(dag.nodes[traits::Observable::id(&y)].is_some());

        y.set(20);
        dag.stabilize();
        assert_eq!(unaffected.observe(), 25);
    }

    /// Bind rewires to a deeper parent; the rebound node fans out into two
    /// parallel paths that both converge on a single downstream node (diamond).
    /// Correct topological height propagation must ensure the shared sink
    /// receives height = max-path-length + 1, not an intermediate value set
    /// by whichever path the traversal happened to visit first.
    #[test]
    fn test_bind_adjust_depth_diamond() {
        let mut dag = Graph::new();

        // shallow: height 0
        let shallow = dag.var(0i32);
        // deep: height 0, but after bind rewire becomes the chosen parent → deeper
        let deep_root = dag.var(0i32);
        // Build a 3-level chain off deep_root so it sits at height 3.
        let d1 = dag.map(&deep_root, |x| x);
        let d2 = dag.map(&d1, |x| x);
        let deep = dag.map(&d2, |x| x); // height 3

        // chooser: picks between shallow (height 0) and deep (height 3)
        let chooser = dag.var(false);
        let shallow_c = shallow.clone();
        let deep_c = deep.clone();
        let binder = dag.bind(&chooser, move |use_deep| {
            if use_deep {
                deep_c.clone().into_input()
            } else {
                shallow_c.clone().into_input()
            }
        });
        // binder initially wired to shallow → height 1

        // Two independent paths from binder converging on a shared sink.
        let arm_a = dag.map(&binder, |x| x);    // height 2
        let arm_b = dag.map(&binder, |x| x);    // height 2
        let sink = dag.map2(&arm_a, &arm_b, |a, b| a + b); // height 3

        let sink_initial_depth = sink.depth();

        // Rewire binder to the deep branch (height 3).
        // binder should move to height 4, arm_a/arm_b to 5, sink to 6.
        chooser.set(true);
        dag.stabilize();

        assert_eq!(arm_a.depth(), sink_initial_depth + 2);
        assert_eq!(arm_b.depth(), sink_initial_depth + 2);
        assert_eq!(sink.depth(), sink_initial_depth + 3);
    }
}
