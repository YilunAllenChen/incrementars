use std::{cell::Ref, marker::PhantomData};

use super::traits::{Incr, IntoInput, Node, Observable, StabilizationResult};

pub(crate) struct _Map3<I1, I2, I3, O> {
    pub(crate) output: Incr<O>,
    pub(crate) input1: Option<Incr<I1>>,
    pub(crate) input2: Option<Incr<I2>>,
    pub(crate) input3: Option<Incr<I3>>,
    pub(crate) f: Option<Box<dyn Fn(I1, I2, I3) -> O>>,
}

impl<I1: Clone + 'static, I2: Clone + 'static, I3: Clone + 'static, O: PartialEq + 'static> Node
    for _Map3<I1, I2, I3, O>
{
    fn stabilize(&mut self) -> StabilizationResult {
        let input1 = self.input1.as_ref().expect("Map3 detached from graph");
        let input2 = self.input2.as_ref().expect("Map3 detached from graph");
        let input3 = self.input3.as_ref().expect("Map3 detached from graph");
        let f = self.f.as_ref().expect("Map3 detached from graph");
        let new_value = f(input1.observe(), input2.observe(), input3.observe());
        let mut output = self.output.state.borrow_mut();
        if new_value == output.value {
            return StabilizationResult::Unchanged;
        }
        output.value = new_value;
        StabilizationResult::Changed
    }

    fn depth(&self) -> i32 {
        self.output.depth()
    }

    fn adjust_depth(&mut self, new_depth: i32) {
        self.output.state.borrow_mut().depth = new_depth;
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
    pub(crate) inner: Incr<O>,
    pub(crate) marker: PhantomData<fn(I1, I2, I3) -> O>,
}

impl<I1, I2, I3, O> Clone for Map3<I1, I2, I3, O> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<I1, I2, I3, O: Clone> Observable<O> for Map3<I1, I2, I3, O> {
    fn id(&self) -> usize {
        self.inner.id()
    }

    fn observe(&self) -> O {
        self.inner.observe()
    }

    fn depth(&self) -> i32 {
        self.inner.depth()
    }
}

impl<I1, I2, I3, O> Map3<I1, I2, I3, O> {
    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, O> {
        self.inner.observe_ref()
    }
}

impl<I1, I2, I3, O: Clone> IntoInput<O> for Map3<I1, I2, I3, O> {
    fn into_input(self) -> Incr<O> {
        self.inner
    }
}

impl<I1, I2, I3, O: Clone> IntoInput<O> for &Map3<I1, I2, I3, O> {
    fn into_input(self) -> Incr<O> {
        self.inner.clone()
    }
}
