use std::{cell::RefCell, rc::Rc};

pub type NodeId = usize;

pub trait Node<T> {
    fn value(&self) -> T;
    fn stablize(&mut self);
    fn children(&self) -> Vec<Rc<RefCell<dyn Node<T>>>>;
}

#[derive(Clone)]
pub struct Var<T>
where
    T: Clone,
{
    pub value: RefCell<T>,
    children: Vec<Rc<RefCell<dyn Node<T>>>>,
}

impl<T> Var<T>
where
    T: Clone,
{
    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
    }

    pub fn make(value: T) -> Rc<RefCell<Var<T>>> {
        Rc::new(RefCell::new(Var {
            value: RefCell::new(value),
            children: vec![],
        }))
    }
}

impl<T> Node<T> for Var<T>
where
    T: Clone,
{
    fn stablize(&mut self) {}
    fn value(&self) -> T {
        self.value.borrow().clone()
    }
    fn children(&self) -> Vec<Rc<RefCell<dyn Node<T>>>> {
        self.children.clone()
    }
}

pub struct Map2<T, U, V>
where
    T: Clone,
    U: Clone,
    V: Clone,
{
    pub value: T,
    pub parents: (Rc<RefCell<dyn Node<U>>>, Rc<RefCell<dyn Node<V>>>),
    pub children: Vec<Rc<RefCell<dyn Node<T>>>>,
    pub func: fn(U, V) -> T,
}

impl<T, U, V> Node<T> for Map2<T, U, V>
where
    T: Clone,
    U: Clone,
    V: Clone,
{
    fn stablize(&mut self) {
        self.value = (self.func)(
            self.parents.0.borrow().value(),
            self.parents.1.borrow().value(),
        );
    }
    fn value(&self) -> T {
        self.value.clone()
    }
    fn children(&self) -> Vec<Rc<RefCell<dyn Node<T>>>> {
        self.children.clone()
    }
}

impl<T, U, V> Map2<T, U, V>
where
    T: Clone,
    U: Clone,
    V: Clone,
{
    pub fn make(
        n1: &Rc<RefCell<impl Node<U> + 'static>>,
        n2: &Rc<RefCell<impl Node<V> + 'static>>,
        func: fn(U, V) -> T,
    ) -> Rc<RefCell<Map2<T, U, V>>> {
        Rc::new(RefCell::new(Map2 {
            value: (func)(n1.borrow().value(), n2.borrow().value()),
            children: vec![],
            parents: (n1.clone(), n2.clone()),
            func,
        }))
    }
}

pub struct Graph<T>
where
    T: Clone,
{
    pub nodes: Vec<Rc<RefCell<dyn Node<T>>>>,
}

impl<T> Graph<T>
where
    T: Clone + 'static,
{
    pub fn new() -> Self {
        Graph { nodes: vec![] }
    }

    pub fn var(&mut self, value: T) -> Rc<RefCell<Var<T>>> {
        let node = Var::make(value);
        self.nodes.push(node.clone());
        node
    }

    pub fn map2<U, V>(
        &mut self,
        n1: &Rc<RefCell<impl Node<U> + 'static>>,
        n2: &Rc<RefCell<impl Node<V> + 'static>>,
        func: fn(U, V) -> T,
    ) -> Rc<RefCell<Map2<T, U, V>>>
    where
        U: Clone + 'static,
        V: Clone + 'static,
    {
        let node = Map2::make(n1, n2, func);
        self.nodes.push(node.clone());
        node
    }

    pub fn stablize(&mut self) {
        self.nodes
            .iter_mut()
            .for_each(|n| n.borrow_mut().stablize());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_stablize() {
        let v1 = Var::make(2);
        let v2 = Var::make(3);
        let m1 = Map2::make(&v1, &v2, |a, b| a + b);
        let m2 = Map2::make(&v1, &m1, |a, b| a * b);
        m1.borrow_mut().stablize();
        m2.borrow_mut().stablize();
        assert_eq!(m1.borrow().value(), 5);
        assert_eq!(m2.borrow().value(), 10);
        v1.borrow_mut().set(4);
        m1.borrow_mut().stablize();
        m2.borrow_mut().stablize();
        assert_eq!(m1.borrow().value(), 7);
        assert_eq!(m2.borrow().value(), 28);
    }

    #[test]
    fn test_graph() {
        let mut g = Graph::new();
        let v1 = g.var(2);
        let v2 = g.var(3);
        let v3 = g.var(5);
        let m1 = g.map2(&v1, &v2, |a, b| a + b);
        let m2 = g.map2(&m1, &v3, |a, b| a * b);
        assert_eq!(m2.borrow().value(), 25);

        v1.borrow_mut().set(4);
        g.stablize();
        assert_eq!(m2.borrow().value(), 35);
    }
}
