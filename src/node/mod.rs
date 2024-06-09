use std::cmp::min;
use std::collections::{BinaryHeap, HashMap};
use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use traits::StablizationCallback;

use self::traits::MaybeDirty;
mod bind;
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

impl<'a> Incrementars<'a> {
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
        f: fn(I) -> Box<dyn Observable<O>>,
    ) -> Bind1<I, O> {
        let id = self.id_counter;
        self.id_counter += 1;
        let input_id = input.id();
        match self.dependencies.get_mut(&input_id) {
            Some(input_deps) => input_deps.push(id),
            None => {
                self.dependencies.insert(input_id, vec![id]);
            }
        }
        let node = Rc::new(RefCell::new(_Bind1 {
            id,
            depth: input.depth() - 1,
            value: (f)(input.observe()),
            input,
            f,
        }));
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

        let mut last_id = usize::MAX;

        while let Some((_h, head_id)) = queue.pop() {
            let node = &self.nodes[head_id];
            if head_id == last_id {
                return;
            }
            last_id = head_id;
            let res = node.deref().borrow_mut().stablize();
            res.into_iter().for_each(|cb| match cb {
                StablizationCallback::ValueChanged => match self.dependencies.get(&head_id) {
                    Some(dependent_ids) => dependent_ids.iter().for_each(|id| {
                        let height = self.nodes[*id].deref().borrow().depth();
                        queue.push((height, *id));
                    }),
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
                        })
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn var_instantiation() {
        let mut compute = Incrementars::new();
        let var = compute.var(0);
        var.set(10);
        assert_eq!(var.observe(), 10);
    }

    #[test]
    fn map_instantiation() {
        let mut compute = Incrementars::new();
        let var = compute.var(0);
        let map = compute.map(Box::new(var), |x| x + 1);
        assert_eq!(map.observe(), 1);
    }

    #[test]
    fn bifurcate() {
        let mut compute = Incrementars::new();
        let var = compute.var(0);
        let map = compute.map(as_input!(var), |x| x + 1);
        let map2 = compute.map(as_input!(var), |x| x + 1);
        assert_eq!(map.observe(), 1);
        assert_eq!(map2.observe(), 1);

        var.set(10);
        assert_eq!(map.observe(), 1);
        compute.stablize();
        assert_eq!(map.observe(), 11);
        assert_eq!(map2.observe(), 11);
    }

    #[test]
    fn test_map2() {
        let mut compute = Incrementars::new();
        let var1 = compute.var(50);
        let var2 = compute.var(" dollars");
        let map2 = compute.map2(as_input!(var1), as_input!(var2), |x, y| x.to_string() + y);
        assert_eq!(map2.observe(), "50 dollars");
    }

    #[test]
    fn test_combinatoric() {
        let mut compute = Incrementars::new();
        let var1 = compute.var(50);
        let plus_one = |x| x + 1;
        let var21 = compute.map(as_input!(var1), plus_one.clone());
        let var22 = compute.map(as_input!(var21), plus_one.clone());
        let var23 = compute.map(as_input!(var22), plus_one.clone());
        let var31 = compute.map(as_input!(var1), plus_one.clone());
        let rejoin = compute.map2(as_input!(var31), as_input!(var23), |x, y| x + y);
        println!("Dependencies: {:?}", compute.dependencies);

        var1.set(10);
        compute.stablize();
        assert_eq!(rejoin.observe(), 24);
    }

    #[test]
    fn test_bind() {
        let mut compute = Incrementars::new();
        let left = compute.var("Left");
        let right = compute.var("Right");

        #[derive(Debug, Clone, Copy)]
        enum Side {
            Left,
            Right,
        }

        let picker = compute.var(Side::Left);


        let binder = compute.bind(as_input!(picker), |x| {
            match x {
                Side::Left => as_input!(left),
                Side::Right => as_input!(right),
            }
        });
    }
}
