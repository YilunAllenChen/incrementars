use std::{cell::RefCell, ops::Deref, rc::Rc};

pub trait Node {
    fn id(&self) -> usize;
    fn height(&self) -> usize;
    fn stablize(&mut self);
}

pub trait Upstream<T> {
    fn observe(&self) -> T;
}

/// Internal representation of a Var node.
pub struct _Var<T> {
    id: usize,
    height: usize,
    value: T,
}

impl<T> Node for _Var<T> {
    fn id(&self) -> usize {
        self.id
    }
    fn height(&self) -> usize {
        self.height
    }
    fn stablize(&mut self) {}
}

impl<T> _Var<T> {
    pub fn new(id: usize, height: usize, value: T) -> Self {
        Self { id, height, value }
    }
}
/// A variable node.
pub struct Var<T> {
    node: Rc<RefCell<_Var<T>>>,
}

impl<T> Clone for Var<T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
        }
    }
}

impl<T> Var<T> {
    pub fn set(&self, value: T) {
        let mut borrowed = self.node.deref().borrow_mut();
        borrowed.value = value;
    }
}

impl<T: Copy> Upstream<T> for Var<T> {
    fn observe(&self) -> T {
        let borrowed = self.node.deref().borrow();
        borrowed.value
    }
}

struct _Map<I, O> {
    id: usize,
    height: usize,
    value: O,
    input: Box<dyn Upstream<I>>,
    f: fn(I) -> O,
}

impl<I, O> Node for _Map<I, O> {
    fn id(&self) -> usize {
        self.id
    }
    fn height(&self) -> usize {
        self.height
    }
    fn stablize(&mut self) {
        self.value = (self.f)(self.input.observe());
    }
}

pub struct Map<I, O> {
    node: Rc<RefCell<_Map<I, O>>>,
}

impl<I, O: Copy> Upstream<O> for Map<I, O> {
    fn observe(&self) -> O {
        let borrowed = self.node.deref().borrow();
        borrowed.value
    }
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

    pub fn map<I: 'a, O: 'a>(&mut self, input: Box<dyn Upstream<I>>, f: fn(I) -> O) -> Map<I, O> {
        let id = self.id_counter;
        self.id_counter += 1;
        let node = Rc::new(RefCell::new(_Map {
            id,
            height: 0,
            value: (f)(input.observe()),
            input,
            f,
        }));
        self.nodes.push(node.clone());
        Map { node }
    }

    pub fn _stablize(&mut self) {}
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
        let map = compute.map(Box::new(var.clone()), |x| x + 1);
        let map2 = compute.map(Box::new(var), |x| x + 1);
        assert_eq!(map.observe(), 1);
        assert_eq!(map2.observe(), 1);
    }
}
