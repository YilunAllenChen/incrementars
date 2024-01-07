use std::{cell::RefCell, rc::Rc};
pub type NodeId = u64;

/**
Trait defines the behaviors of a node.
*/
pub trait NodeBehavior {
    fn id(&self) -> NodeId;
    fn stablize(&mut self);
    fn dirty(&self) -> bool;
}

pub type NodeIdentityHandle<T> = Rc<RefCell<T>>;
pub type NodeInputHandle<T> = Rc<RefCell<dyn NodeInput<T>>>;
pub type NodeBehaviorHandle = Rc<RefCell<dyn NodeBehavior>>;

/**
Trait defines the value of a node. This needs to be separate from the
NodeBehavior trait because Value is parameterized by the value type of the node,
whereas NodeBehavior is agnostic.
*/
pub trait NodeValue<T> {
    fn value(&self) -> T;
}

pub trait NodeInput<In>: NodeBehavior + NodeValue<In> {}

/**
Creates read/write handles for a node in a triple: (ident, input, behavior).
- ident: The identifier of the node, useful especially for a Var node such that we can write to it.
- input: The input handle, useful when passing into another node as a dependency / input.
- behavior: The behavior handle, useful when we don't want the type parameter an just need to guarantee the behavior.
*/
pub fn handles<T, U>(
    value: T,
) -> (
    NodeIdentityHandle<T>,
    NodeInputHandle<U>,
    NodeBehaviorHandle,
)
where
    T: NodeInput<U> + 'static,
{
    let rc = Rc::new(RefCell::new(value));
    (rc.clone(), rc.clone(), rc)
}
