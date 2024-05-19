use std::cmp::max;
use std::{cell::RefCell, ops::Deref, rc::Rc};

pub use self::{
    map::{Map1, _Map1},
    map2::{Map2, _Map2},
    traits::{Node, Observable},
    var::{Var, _Var},
};
mod map;
mod map2;
mod traits;
mod var;

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
}

impl<'a> Incrementars<'a> {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            id_counter: 0,
        }
    }

    pub fn var<T: Copy + 'a>(&mut self, value: T) -> Var<T> {
        let id = self.id_counter;
        self.id_counter += 1;
        let node = Rc::new(RefCell::new(_Var::new(id, 0, value)));
        self.nodes.push(node.clone());
        Var { node }
    }

    pub fn map<I: 'a, O: 'a>(
        &mut self,
        input: Box<dyn Observable<I>>,
        f: fn(I) -> O,
    ) -> Map1<I, O> {
        let id = self.id_counter;
        self.id_counter += 1;
        let node = Rc::new(RefCell::new(_Map1 {
            id,
            height: input.height() + 1,
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
        let node = Rc::new(RefCell::new(_Map2 {
            id,
            height: max(input1.height(), input2.height()) + 1,
            value: (f)(input1.observe(), input2.observe()),
            input1,
            input2,
            f,
        }));
        self.nodes.push(node.clone());
        Map2 { node }
    }

    pub fn stablize(&mut self) {
        self.nodes.sort_by_key(|x| x.deref().borrow().height());
        self.nodes
            .iter()
            .for_each(|x| x.deref().borrow_mut().stablize());
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
}
