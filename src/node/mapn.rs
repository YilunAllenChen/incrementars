use std::ops::Deref;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use super::traits::{IntoInput, Node, Observable, StabilizationCallback};

pub(crate) struct _MapN<T, O> {
    pub(crate) id: usize,
    pub(crate) depth: i32,
    pub(crate) value: O,
    pub(crate) inputs: Option<Vec<Box<dyn Observable<T>>>>,
    pub(crate) f: Option<Box<dyn Fn(Vec<T>) -> O>>,
}

impl<T: Clone + 'static, O: PartialEq + 'static> Node for _MapN<T, O> {
    fn id(&self) -> usize {
        self.id
    }
    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        let inputs = self.inputs.as_ref().expect("MapN detached from graph");
        let f = self.f.as_ref().expect("MapN detached from graph");
        let new_value = f(inputs.iter().map(|input| input.observe()).collect());
        if new_value == self.value {
            return vec![];
        }
        self.value = new_value;
        vec![StabilizationCallback::ValueChanged]
    }
    fn depth(&self) -> i32 {
        self.depth
    }
    fn adjust_depth(&mut self, new_depth: i32) {
        self.depth = new_depth;
    }
    fn teardown(&mut self) {
        self.inputs = None;
        self.f = None;
    }
}

/// A node that combines a homogeneous list of upstream values through a function.
/// Create with [`Incrementars::mapn`].
///
/// Cloning a `MapN` produces a second handle to the same node.
pub struct MapN<T, O> {
    pub(crate) node: Rc<RefCell<_MapN<T, O>>>,
}

impl<T, O: Clone> Observable<O> for MapN<T, O> {
    fn id(&self) -> usize {
        self.node.deref().borrow().id
    }
    fn observe(&self) -> O {
        self.node.deref().borrow().value.clone()
    }
    fn depth(&self) -> i32 {
        self.node.deref().borrow().depth
    }
}

impl<T, O> Clone for MapN<T, O> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
        }
    }
}

impl<T, O: Clone + 'static> MapN<T, O> {
    /// Returns a boxed clone of this handle, suitable for passing to
    /// [`Incrementars::map`], [`Incrementars::map2`], [`Incrementars::map3`],
    /// [`Incrementars::mapn`], or [`Incrementars::bind`].
    pub fn as_input(&self) -> Box<MapN<T, O>> {
        Box::new(self.clone())
    }
}

impl<T, O> MapN<T, O> {
    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, O> {
        Ref::map(self.node.deref().borrow(), |internal| &internal.value)
    }
}

impl<T: 'static, O: Clone + 'static> IntoInput<O> for MapN<T, O> {
    fn into_input(self) -> Box<dyn Observable<O>> {
        Box::new(self)
    }
}

impl<T: 'static, O: Clone + 'static> IntoInput<O> for Box<MapN<T, O>> {
    fn into_input(self) -> Box<dyn Observable<O>> {
        self
    }
}
