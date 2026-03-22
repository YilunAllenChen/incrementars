use std::cmp::min;
use std::collections::{BinaryHeap, HashMap};
use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use bitmap::Bitmap;
use traits::StabilizationCallback;
use self::traits::MaybeDirty;

mod bind;
mod bitmap;
mod map;
mod map2;
mod traits;
mod var;

pub use self::{
    bind::Bind1,
    map::Map1,
    map2::Map2,
    traits::Observable,
    var::Var,
};

// Vars are assigned the maximum depth so that derived nodes, which subtract 1
// per level, are always processed after their inputs in the stabilization queue.
const VAR_DEPTH: i32 = 1_000;

/// The incremental computation graph.
///
/// Create input nodes with [`var`](Incrementars::var), wire them together with
/// [`map`](Incrementars::map), [`map2`](Incrementars::map2), and
/// [`bind`](Incrementars::bind), then call [`stabilize`](Incrementars::stabilize)
/// to propagate pending changes through the graph.
///
/// # Example
/// ```
/// use incrementars::prelude::*;
/// let mut dag = Incrementars::new();
/// let x = dag.var(2);
/// let y = dag.map(x.as_input(), |v| v * v);
/// assert_eq!(y.observe(), 4);
/// x.set(3);
/// dag.stabilize();
/// assert_eq!(y.observe(), 9);
/// ```
pub struct Incrementars {
    // node id → node handle. IDs are assigned sequentially from 0 by next_id(),
    // so the map always contains exactly the keys 0..id_counter.
    nodes: HashMap<usize, Rc<RefCell<dyn traits::Node>>>,
    id_counter: usize,

    inputs: Vec<Box<dyn MaybeDirty>>,
    // parent_id → [child_ids]: which nodes depend on a given node
    pub(crate) dependencies: HashMap<usize, Vec<usize>>,
    // child_id → [parent_ids]: which nodes a given node depends on
    reverse_dependencies: HashMap<usize, Vec<usize>>,
}

impl Incrementars {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            id_counter: 0,
            inputs: vec![],
            dependencies: HashMap::new(),
            reverse_dependencies: HashMap::new(),
        }
    }

    fn next_id(&mut self) -> usize {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }

    /// Adds a directed edge parent → child to both dependency maps.
    /// Silently deduplicates: calling with the same pair twice has no effect.
    fn add_edge(&mut self, parent_id: usize, child_id: usize) {
        let deps = self.dependencies.entry(parent_id).or_default();
        if !deps.contains(&child_id) {
            deps.push(child_id);
        }
        let rev = self.reverse_dependencies.entry(child_id).or_default();
        if !rev.contains(&parent_id) {
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
        self.nodes.insert(id, node.clone());
        self.inputs.push(Box::new(Var { node: node.clone() }));
        Var { node }
    }

    /// Creates a node that applies `f` to the output of `input` during stabilization.
    ///
    /// `f` is only called when `input` has changed since the last stabilization, and
    /// its result is only propagated downstream if it differs from the previous output
    /// (cutoff optimization, requires `O: PartialEq`).
    pub fn map<I: 'static, O: PartialEq + 'static>(
        &mut self,
        input: Box<dyn Observable<I>>,
        f: impl Fn(I) -> O + 'static,
    ) -> Map1<I, O> {
        let id = self.next_id();
        let input_id = input.id();
        self.add_edge(input_id, id);
        let node = Rc::new(RefCell::new(map::_Map1 {
            id,
            depth: input.depth() - 1,
            value: f(input.observe()),
            input,
            f: Box::new(f),
        }));
        self.nodes.insert(id, node.clone());
        Map1 { node }
    }

    /// Creates a node that applies `f` to the outputs of `input1` and `input2`
    /// during stabilization.
    ///
    /// `f` is called when either input has changed, and the result is only propagated
    /// if it differs from the previous output (cutoff optimization).
    pub fn map2<I1: 'static, I2: 'static, O: PartialEq + 'static>(
        &mut self,
        input1: Box<dyn Observable<I1>>,
        input2: Box<dyn Observable<I2>>,
        f: impl Fn(I1, I2) -> O + 'static,
    ) -> Map2<I1, I2, O> {
        let id = self.next_id();
        let (id1, id2) = (input1.id(), input2.id());
        self.add_edge(id1, id);
        self.add_edge(id2, id);
        let node = Rc::new(RefCell::new(map2::_Map2 {
            id,
            depth: min(input1.depth(), input2.depth()) - 1,
            value: f(input1.observe(), input2.observe()),
            input1,
            input2,
            f: Box::new(f),
        }));
        self.nodes.insert(id, node.clone());
        Map2 { node }
    }

    /// Creates a node whose upstream dependency can change dynamically.
    ///
    /// `f` is called with the current value of `input` to select which node to read
    /// from. When `input` changes and `f` returns a different node, the graph is
    /// rewired and node depths are recalculated automatically.
    pub fn bind<I: 'static, O: 'static>(
        &mut self,
        input: Box<dyn Observable<I>>,
        f: impl Fn(I) -> Box<dyn Observable<O>> + 'static,
    ) -> Bind1<I, O> {
        let id = self.next_id();
        let input_id = input.id();
        let value = f(input.observe());
        let value_id = value.id();
        let depth = min(input.depth(), value.depth()) - 1;
        let node = Rc::new(RefCell::new(bind::_Bind1 {
            id,
            depth,
            value,
            input,
            f: Box::new(f),
        }));
        self.add_edge(input_id, id);
        self.add_edge(value_id, id);
        self.nodes.insert(id, node.clone());
        Bind1 { node }
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
                let node = self.nodes[&id].deref().borrow();
                (node.depth(), node.id())
            })
            .collect::<BinaryHeap<(i32, usize)>>();

        let mut visited = Bitmap::new(self.nodes.len());

        while let Some((_depth, head_id)) = queue.pop() {
            let callbacks = self.nodes[&head_id].deref().borrow_mut().stabilize();

            for cb in callbacks {
                match cb {
                    StabilizationCallback::ValueChanged => {
                        if let Some(children) = self.dependencies.get(&head_id) {
                            for &child_id in children {
                                if !visited.contains(&child_id) {
                                    visited.insert(child_id);
                                    let depth = self.nodes[&child_id].deref().borrow().depth();
                                    queue.push((depth, child_id));
                                }
                            }
                        }
                    }
                    StabilizationCallback::DependenciesUpdated { from, to } => {
                        // Remove old dependency edges
                        for from_id in &from {
                            if let Some(deps) = self.dependencies.get_mut(from_id) {
                                deps.retain(|&x| x != head_id);
                            }
                            if let Some(rev) = self.reverse_dependencies.get_mut(&head_id) {
                                rev.retain(|&x| x != *from_id);
                            }
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
                            let min_parent_depth = self
                                .reverse_dependencies
                                .get(&node_id)
                                .and_then(|parents| {
                                    parents
                                        .iter()
                                        .map(|&pid| self.nodes[&pid].borrow().depth())
                                        .min()
                                });

                            if let Some(parent_depth) = min_parent_depth {
                                let new_depth = parent_depth - 1;
                                let old_depth = self.nodes[&node_id].borrow().depth();
                                if new_depth != old_depth {
                                    self.nodes[&node_id].borrow_mut().adjust_depth(new_depth);
                                    if let Some(children) = self.dependencies.get(&node_id) {
                                        adjust_queue.extend(children.iter().copied());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Default for Incrementars {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let map = dag.map(Box::new(var), |x| x + 1);
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

        let binder = dag.bind(
            picker.as_input(),
            pick(left.as_input(), right.as_input()),
        );
        let binder_id = binder.id();

        assert_eq!(
            dag.dependencies.get(&left_id),
            Some(vec![binder.id()]).as_ref()
        );
        assert_eq!(dag.dependencies.get(&right_id), None);
        assert_eq!(binder.observe(), 1);
        picker.set(Side::Right);
        dag.stabilize();
        assert_eq!(binder.observe(), 2);
        assert_eq!(
            dag.dependencies.get(&right_id),
            Some(vec![binder_id]).as_ref()
        );
        assert_eq!(dag.dependencies.get(&left_id), Some(vec![]).as_ref());
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
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

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
}
