use std::{
    borrow::Borrow,
    cell::{Ref, RefCell},
    ops::Deref,
    rc::Rc,
};

pub mod prelude {}

type NodeID = usize;

pub trait Node {
    fn id(&self) -> NodeID;
    fn depth(&self);
    fn stablize(&self);
}

pub trait Observable<T> {
    fn id(&self) -> NodeID;
    fn observe(&self) -> &T;
}

pub struct Var<T> {
    id: NodeID,
    value: T,
}

impl<T> Var<T> {
    fn new(id: NodeID, value: T) -> Self {
        Self { id, value }
    }
}

impl<T> Observable<T> for Var<T> {
    fn id(&self) -> NodeID {
        self.id
    }
    fn observe(&self) -> &T {
        &self.value
    }
}

pub struct Map1<'a, I: 'a, O> {
    id: usize,
    fun: &'a dyn Fn(&I) -> O,
    value: O,
    input: Rc<RefCell<dyn Observable<I>>>,
}

impl<'a, T: 'a, I> Observable<T> for Map1<'a, I, T> {
    fn id(&self) -> NodeID {
        self.id
    }
    fn observe(&self) -> &T {
        &self.value
    }
}

impl<'a, I, O> Map1<'a, I, O> {
    fn new(id: NodeID, fun: &'a dyn Fn(&I) -> O, input: Rc<RefCell<dyn Observable<I>>>) -> Self {
        let in_clone = input.clone();
        let borrowed = in_clone.deref().borrow();
        let value = fun(borrowed.observe());
        Self {
            id,
            fun,
            value,
            input,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Map1;

    use super::*;
    #[test]
    fn var_instantiation() {
        let _ = Var::new(1, 10);
    }

    #[test]
    fn map_instantiation() {
        let var = Var::new(1, 10);
        let var_rc = Rc::new(RefCell::new(var));
        let map1 = Map1::new(2, &|x| x + 1, var_rc);
    }
}
