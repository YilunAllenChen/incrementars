use std::{cell::Ref, marker::PhantomData};

use super::traits::{Incr, IntoInput, Node, Observable, StabilizationCallback};

pub(crate) struct _Map2<I1, I2, O> {
    pub(crate) output: Incr<O>,
    pub(crate) input1: Option<Incr<I1>>,
    pub(crate) input2: Option<Incr<I2>>,
    pub(crate) f: Option<Box<dyn Fn(I1, I2) -> O>>,
}

impl<I1: Clone + 'static, I2: Clone + 'static, O: PartialEq + 'static> Node for _Map2<I1, I2, O> {
    fn id(&self) -> usize {
        self.output.id()
    }

    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        let input1 = self.input1.as_ref().expect("Map2 detached from graph");
        let input2 = self.input2.as_ref().expect("Map2 detached from graph");
        let f = self.f.as_ref().expect("Map2 detached from graph");
        let new_value = f(input1.observe(), input2.observe());
        let mut output = self.output.state.borrow_mut();
        if new_value == output.value {
            return vec![];
        }
        output.value = new_value;
        vec![StabilizationCallback::ValueChanged]
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
        self.f = None;
    }
}

/// A node that combines two upstream values through a function. Create with [`Incrementars::map2`].
///
/// Cloning a `Map2` produces a second handle to the same node.
pub struct Map2<I1, I2, O> {
    pub(crate) inner: Incr<O>,
    pub(crate) marker: PhantomData<fn(I1, I2) -> O>,
}

impl<I1, I2, O> Clone for Map2<I1, I2, O> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<I1, I2, O: Clone> Observable<O> for Map2<I1, I2, O> {
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

impl<I1, I2, O> Map2<I1, I2, O> {
    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, O> {
        self.inner.observe_ref()
    }
}

impl<I1, I2, O: Clone> IntoInput<O> for Map2<I1, I2, O> {
    fn into_input(self) -> Incr<O> {
        self.inner
    }
}

impl<I1, I2, O: Clone> IntoInput<O> for &Map2<I1, I2, O> {
    fn into_input(self) -> Incr<O> {
        self.inner.clone()
    }
}
