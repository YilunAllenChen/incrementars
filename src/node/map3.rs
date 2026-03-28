use std::ops::Deref;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use super::traits::{IntoInput, Node, Observable, StabilizationCallback};

pub(crate) struct _Map3<I1, I2, I3, O> {
    pub(crate) id: usize,
    pub(crate) depth: i32,
    pub(crate) value: O,
    pub(crate) input1: Option<Box<dyn Observable<I1>>>,
    pub(crate) input2: Option<Box<dyn Observable<I2>>>,
    pub(crate) input3: Option<Box<dyn Observable<I3>>>,
    pub(crate) f: Option<Box<dyn Fn(I1, I2, I3) -> O>>,
}

impl<I1: 'static, I2: 'static, I3: 'static, O: PartialEq + 'static> Node for _Map3<I1, I2, I3, O> {
    fn id(&self) -> usize {
        self.id
    }
    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        let input1 = self.input1.as_ref().expect("Map3 detached from graph");
        let input2 = self.input2.as_ref().expect("Map3 detached from graph");
        let input3 = self.input3.as_ref().expect("Map3 detached from graph");
        let f = self.f.as_ref().expect("Map3 detached from graph");
        let new_value = f(input1.observe(), input2.observe(), input3.observe());
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
        self.input1 = None;
        self.input2 = None;
        self.input3 = None;
        self.f = None;
    }
}

/// A node that combines three upstream values through a function. Create with [`Incrementars::map3`].
///
/// Cloning a `Map3` produces a second handle to the same node.
pub struct Map3<I1, I2, I3, O> {
    pub(crate) node: Rc<RefCell<_Map3<I1, I2, I3, O>>>,
}

impl<I1, I2, I3, O: Clone> Observable<O> for Map3<I1, I2, I3, O> {
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

impl<I1, I2, I3, O> Clone for Map3<I1, I2, I3, O> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
        }
    }
}

impl<I1, I2, I3, O: Clone + 'static> Map3<I1, I2, I3, O> {
    /// Returns a boxed clone of this handle, suitable for passing to
    /// [`Incrementars::map`], [`Incrementars::map2`], [`Incrementars::map3`],
    /// [`Incrementars::mapn`], or [`Incrementars::bind`].
    pub fn as_input(&self) -> Box<Map3<I1, I2, I3, O>> {
        Box::new(self.clone())
    }
}

impl<I1, I2, I3, O> Map3<I1, I2, I3, O> {
    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, O> {
        Ref::map(self.node.deref().borrow(), |internal| &internal.value)
    }
}

impl<I1: 'static, I2: 'static, I3: 'static, O: Clone + 'static> IntoInput<O>
    for Map3<I1, I2, I3, O>
{
    fn into_input(self) -> Box<dyn Observable<O>> {
        Box::new(self)
    }
}

impl<I1: 'static, I2: 'static, I3: 'static, O: Clone + 'static> IntoInput<O>
    for Box<Map3<I1, I2, I3, O>>
{
    fn into_input(self) -> Box<dyn Observable<O>> {
        self
    }
}
