use std::cmp::min;
use std::collections::{BinaryHeap, HashMap};
use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use bitmap::Bitmap;

use traits::StablizationCallback;

use self::traits::MaybeDirty;
mod bind;
mod bitmap;
mod map;
mod map2;
mod traits;
mod var;
pub use self::{
    bind::{Bind1, _Bind1},
    map::{Map1, _Map1},
    map2::{Map2, _Map2},
    traits::{Node, Observable},
    var::{Var, _Var},
};

/// Actualy just creating a Box from a clone. Equivalent to `Box::new($e.clone())`
#[macro_export]
macro_rules! as_input {
    ($e:expr) => {
        Box::new($e.clone())
    };
}

pub struct Incrementars<'a> {
    nodes: Vec<Rc<RefCell<dyn Node + 'a>>>,
    id_counter: usize,

    inputs: Vec<Box<dyn MaybeDirty + 'a>>,
    // key is node id, value is list of node ids that depend on the node.
    dependencies: HashMap<usize, Vec<usize>>,
}

impl<'a: 'static> Incrementars<'a> {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            id_counter: 0,
            inputs: vec![],
            dependencies: HashMap::new(),
        }
    }

    pub fn var<T: Copy + 'a>(&mut self, value: T) -> Var<T> {
        let id = self.id_counter;
        self.id_counter += 1;
        // max height for dag is 1000.
        let node = Rc::new(RefCell::new(_Var::new(id, 1_000, value)));
        self.nodes.push(node.clone());
        self.inputs.push(Box::new(Var { node: node.clone() }));
        Var { node }
    }

    pub fn map<I: 'a, O: 'a>(
        &mut self,
        input: Box<dyn Observable<I>>,
        f: fn(I) -> O,
    ) -> Map1<I, O> {
        let id = self.id_counter;
        self.id_counter += 1;
        let input_id = input.id();
        match self.dependencies.get_mut(&input_id) {
            Some(input_deps) => input_deps.push(id),
            None => {
                self.dependencies.insert(input_id, vec![id]);
            }
        }
        let node = Rc::new(RefCell::new(_Map1 {
            id,
            depth: input.depth() - 1,
            value: (f)(input.observe()),
            input,
            f,
        }));
        self.nodes.push(node.clone());
        Map1 { node }
    }

    pub fn map2<I1: 'a, I2: 'a, O: 'a>(
        &mut self,
        input1: Box<dyn Observable<I1>>,
        input2: Box<dyn Observable<I2>>,
        f: fn(I1, I2) -> O,
    ) -> Map2<I1, I2, O> {
        let id = self.id_counter;
        self.id_counter += 1;
        for input_id in [input1.id(), input2.id()] {
            match self.dependencies.get_mut(&input_id) {
                Some(input_deps) => input_deps.push(id),
                None => {
                    self.dependencies.insert(input_id, vec![id]);
                }
            }
        }
        let node = Rc::new(RefCell::new(_Map2 {
            id,
            depth: min(input1.depth(), input2.depth()) - 1,
            value: (f)(input1.observe(), input2.observe()),
            input1,
            input2,
            f,
        }));
        self.nodes.push(node.clone());
        Map2 { node }
    }

    pub fn bind<I: 'a, O: 'a>(
        &mut self,
        input: Box<dyn Observable<I>>,
        f: Box<impl Fn(I) -> Box<dyn Observable<O>> + 'a>,
    ) -> Bind1<I, O> {
        let id = self.id_counter;
        self.id_counter += 1;
        let input_id = input.id();
        let value = (f)(input.observe());
        let value_id = value.id();
        let depth = min(input.depth(), value.depth()) - 1;
        let node = Rc::new(RefCell::new(_Bind1 {
            id,
            depth,
            value,
            input,
            f,
        }));
        [input_id, value_id]
            .iter()
            .for_each(|x| match self.dependencies.get_mut(x) {
                Some(deps) => deps.push(id),
                None => {
                    self.dependencies.insert(*x, vec![id]);
                }
            });
        self.nodes.push(node.clone());
        Bind1 { node }
    }

    pub fn stablize(&mut self) {
        let dirty_inputs = self.inputs.iter().filter(|x| x.is_dirty());

        let mut queue = dirty_inputs
            .map(|x| x.id())
            .map(|id| self.nodes.get(id).unwrap().deref().borrow())
            .map(|node| (node.depth(), node.id()))
            .collect::<BinaryHeap<(i32, usize)>>();

        let mut visited = Bitmap::new(self.nodes.len());

        while let Some((_h, head_id)) = queue.pop() {
            let node = &self.nodes[head_id];
            let res = node.deref().borrow_mut().stablize();
            res.into_iter().for_each(|cb| match cb {
                StablizationCallback::ValueChanged => match self.dependencies.get(&head_id) {
                    Some(dependent_ids) => {
                        dependent_ids.iter().for_each(|id| {
                            // because pseudoheight guarantees that all nodes must fire *after* all
                            // of its dependencies fire, node needs to only be fired once. Skip if
                            // we have already visited this node.
                            if !visited.contains(id) {
                                visited.insert(*id);
                                let depth = self.nodes[*id].deref().borrow().depth();
                                queue.push((depth, *id));
                            }
                        })
                    }
                    None => {}
                },
                StablizationCallback::DependenciesUpdated { from, to } => {
                    from.iter()
                        .for_each(|id| match self.dependencies.get_mut(id) {
                            Some(deps) => {
                                deps.retain(|x| *x != head_id);
                            }
                            None => {}
                        });
                    to.iter()
                        .for_each(|id| match self.dependencies.get_mut(id) {
                            Some(deps) => {
                                deps.push(head_id);
                            }
                            None => {
                                self.dependencies.insert(*id, vec![head_id]);
                            }
                        });

                    let adjust_start_id = node.deref().borrow().id();
                    let mut adjust_queue = vec![adjust_start_id];

                    while let Some(node_id) = adjust_queue.pop() {
                        let min_upstream_depth = self
                            .dependencies
                            .iter()
                            .filter_map(|(other, deps)| {
                                if deps.contains(&node_id) {
                                    Some(self.nodes.get(*other).unwrap().deref().borrow().depth())
                                } else {
                                    None
                                }
                            })
                            .min();

                        if let Some(raw_depth) = min_upstream_depth {
                            let old_depth =
                                self.nodes.get(node_id).unwrap().deref().borrow().depth();
                            let new_depth = raw_depth - 1;
                            if new_depth < old_depth {
                                self.nodes
                                    .get(node_id)
                                    .unwrap()
                                    .borrow_mut()
                                    .adjust_depth(new_depth);
                                let this_node_id = node.deref().borrow().id();
                                if let Some(dependencies) = self.dependencies.get(&this_node_id) {
                                    adjust_queue.extend(dependencies);
                                }
                            }
                        }
                    }
                }
            })
        }
    }

    pub fn print(&self) {
        self.dependencies.iter().for_each(|(id, deps)| {
            println!("dep | {:?} depends on {}", deps, id);
        });
        self.nodes.iter().for_each(|node| {
            let bor = node.deref().borrow();
            println!("node | {} @ {}", bor.id(), bor.depth());
        })
    }
}

#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;

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
        let map = dag.map(as_input!(var), |x| x + 1);
        let map2 = dag.map(as_input!(var), |x| x + 1);
        assert_eq!(map.observe(), 1);
        assert_eq!(map2.observe(), 1);

        var.set(10);
        assert_eq!(map.observe(), 1);
        dag.stablize();
        assert_eq!(map.observe(), 11);
        assert_eq!(map2.observe(), 11);
    }

    #[test]
    fn test_map2() {
        let mut dag = Incrementars::new();
        let var1 = dag.var(50);
        let var2 = dag.var(" dollars");
        let map2 = dag.map2(as_input!(var1), as_input!(var2), |x, y| x.to_string() + y);
        assert_eq!(map2.observe(), "50 dollars");
    }

    #[test]
    fn test_combinatoric() {
        let mut dag = Incrementars::new();
        let var1 = dag.var(50);
        let plus_one = |x| x + 1;
        let var21 = dag.map(as_input!(var1), plus_one.clone());
        let var22 = dag.map(as_input!(var21), plus_one.clone());
        let var23 = dag.map(as_input!(var22), plus_one.clone());
        let var31 = dag.map(as_input!(var1), plus_one.clone());
        let rejoin = dag.map2(as_input!(var31), as_input!(var23), |x, y| x + y);

        var1.set(10);
        dag.stablize();
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
            as_input!(picker),
            Box::new(pick(as_input!(left), as_input!(right))),
        );
        let binder_id = binder.id();

        assert_eq!(
            dag.dependencies.get(&left_id),
            Some(vec![binder.id()]).as_ref()
        );
        assert_eq!(dag.dependencies.get(&right_id), None);
        assert_eq!(binder.observe(), 1);
        picker.set(Side::Right);
        dag.stablize();
        assert_eq!(binder.observe(), 2);
        assert_eq!(
            dag.dependencies.get(&right_id),
            Some(vec![binder_id]).as_ref()
        );
        assert_eq!(dag.dependencies.get(&left_id), Some(vec![]).as_ref());
    }

    #[test]
    fn test_bind_adjust_height_propagaion() {
        let mut dag = Incrementars::new();
        let left_root = dag.var(1);
        let right_root = dag.var(-1);
        let left_map = dag.map(as_input!(left_root), |x| x * 2);

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
            as_input!(picker),
            Box::new(pick(as_input!(left_map), as_input!(right_root))),
        );

        let map_after_bind = dag.map(as_input!(binder), |n| n * 10);
        let binder_old_depth = binder.depth();
        let mabind_old_depth = map_after_bind.depth();

        dag.print();
        picker.set(Side::Left);
        dag.stablize();
        dag.print();
        let binder_new_depth = binder.depth();
        let mabind_new_depth = map_after_bind.depth();

        assert_eq!(binder_new_depth, binder_old_depth - 1);
        assert_eq!(mabind_new_depth, mabind_old_depth - 1);
    }

    #[test]
    fn test_real_life() {
        let mut dag = Incrementars::new();
        let length = dag.var(2.0);
        let area = dag.map(as_input!(length), |x| x * x);

        // on initial stabalization, area is calculated to be 4.
        assert_eq!(area.observe(), 4.0);
        length.set(3.0);

        // right after setting, dag isn't stablized yet.
        assert_eq!(area.observe(), 4.0);

        dag.stablize();
        assert_eq!(area.observe(), 9.0);

        let height = dag.var(5.0);
        let volume = dag.map2(as_input!(area), as_input!(height), |x, y| x * y);

        assert_eq!(volume.observe(), 45.0);

        height.set(10.0);
        dag.stablize();
        assert_eq!(volume.observe(), 90.0);
    }

    #[test]
    fn test_combinatorical_only_fire_once_at_combine() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        // Define a global static AtomicUsize counter
        lazy_static! {
            static ref GLOBAL_COUNTER: AtomicUsize = AtomicUsize::new(0);
        }

        fn increment_counter() {
            GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst);
        }

        fn get_counter() -> usize {
            GLOBAL_COUNTER.load(Ordering::SeqCst)
        }

        let mut dag = Incrementars::new();
        let var1 = dag.var(1);
        let plus_one = |x| x + 1;
        let left1 = dag.map(as_input!(var1), plus_one.clone());
        let left2 = dag.map(as_input!(left1), plus_one.clone());
        let left3 = dag.map(as_input!(left2), plus_one.clone());

        let right = dag.map(as_input!(var1), plus_one.clone());

        fn incr_counter(_: i32, _: i32) {
            increment_counter();
        }

        dag.map2(as_input!(left3), as_input!(right), incr_counter);
        assert_eq!(get_counter(), 1);
    }
}
