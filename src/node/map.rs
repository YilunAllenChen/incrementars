use std::{cell::Ref, marker::PhantomData};

use super::traits::{Incr, IntoInput, Node, Observable, StabilizationCallback};

pub(crate) struct _Map1<I, O> {
    pub(crate) output: Incr<O>,
    pub(crate) input: Option<Incr<I>>,
    pub(crate) f: Option<Box<dyn Fn(I) -> O>>,
}

impl<I: Clone + 'static, O: PartialEq + 'static> Node for _Map1<I, O> {
    fn id(&self) -> usize {
        self.output.id()
    }

    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        let input = self.input.as_ref().expect("Map1 detached from graph");
        let f = self.f.as_ref().expect("Map1 detached from graph");
        let new_value = f(input.observe());
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
        self.input = None;
        self.f = None;
    }
}

/// A node that maps one upstream value through a function. Create with [`Incrementars::map`].
///
/// Cloning a `Map1` produces a second handle to the same node.
pub struct Map1<I, O> {
    pub(crate) inner: Incr<O>,
    pub(crate) marker: PhantomData<fn(I) -> O>,
}

impl<I, O> Clone for Map1<I, O> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<I, O: Clone> Observable<O> for Map1<I, O> {
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

impl<I, O> Map1<I, O> {
    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, O> {
        self.inner.observe_ref()
    }
}

impl<I, O: Clone> IntoInput<O> for Map1<I, O> {
    fn into_input(self) -> Incr<O> {
        self.inner
    }
}

impl<I, O: Clone> IntoInput<O> for &Map1<I, O> {
    fn into_input(self) -> Incr<O> {
        self.inner.clone()
    }
}
