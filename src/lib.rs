use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{cell::RefCell, ops::Deref, rc::Rc};

type NodeID = usize;

static GLOBAL_COUNTER: Lazy<AtomicUsize> = Lazy::new(|| AtomicUsize::new(0));

fn next_id() -> usize {
    let id = GLOBAL_COUNTER.load(Ordering::SeqCst);
    GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst);
    id
}

pub trait Node<O> {
    fn id(&self) -> NodeID;
    fn observe(&self) -> &O;
    fn stablize(&mut self);
    fn height(&self) -> usize;
}

pub fn var<O>(value: O) -> Rc<RefCell<Var<O>>> {
    Rc::new(RefCell::new(Var {
        id: next_id(),
        value,
        dirty: false,
    }))
}

/// Just peek at the value, not enforcing stablization
pub fn peek<T: Copy>(node: Rc<RefCell<dyn Node<T>>>) -> T {
    *node.deref().borrow().observe()
}

pub fn stablize<T>(node: Rc<RefCell<dyn Node<T>>>) {
    node.deref().borrow_mut().stablize();
}

pub fn map1<I, O>(input: Rc<RefCell<dyn Node<I>>>, fun: fn(&I) -> O) -> Rc<RefCell<Map1<I, O>>> {
    let value = fun(&input.deref().borrow().observe());
    let height = input.borrow().deref().height() - 1;
    Rc::new(RefCell::new(Map1 {
        id: next_id(),
        fun,
        height,
        value,
        input,
    }))
}

pub struct Var<T> {
    id: NodeID,
    value: T,
    dirty: bool,
}
impl<T> Node<T> for Var<T> {
    fn id(&self) -> NodeID {
        self.id
    }
    fn observe(&self) -> &T {
        &self.value
    }
    fn stablize(&mut self) {
        self.dirty = false
    }
    fn height(&self) -> usize {
        usize::MAX
    }
}

impl<T> Var<T> {
    pub fn set(&mut self, new: T) {
        self.value = new
    }
}

pub struct Map1<I, O> {
    id: usize,
    fun: fn(&I) -> O,
    height: usize,
    value: O,
    input: Rc<RefCell<dyn Node<I>>>,
}

impl<T, I> Node<T> for Map1<I, T> {
    fn id(&self) -> NodeID {
        self.id
    }
    fn observe(&self) -> &T {
        &self.value
    }
    fn stablize(&mut self) {
        self.input.deref().borrow_mut().stablize();
        self.value = (self.fun)(self.input.deref().borrow().observe())
    }
    fn height(&self) -> usize {
        self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn var_instantiation() {
        let _ = var(10);
    }

    #[test]
    fn map_instantiation() {
        let var = var(10);
        let _var_rc = map1(var, |x| x + 1);
    }
}
