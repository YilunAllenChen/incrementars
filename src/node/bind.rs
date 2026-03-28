use std::{cell::Ref, marker::PhantomData};

use super::traits::{Incr, IntoInput, Node, Observable, StabilizationCallback};

pub(crate) struct _Bind1<I, O> {
    pub(crate) output: Incr<O>,
    pub(crate) value: Option<Incr<O>>,
    pub(crate) input: Option<Incr<I>>,
    pub(crate) f: Option<Box<dyn Fn(I) -> Incr<O>>>,
}

impl<I: Clone + 'static, O: Clone + PartialEq + 'static> Node for _Bind1<I, O> {
    fn id(&self) -> usize {
        self.output.id()
    }

    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        let input = self.input.as_ref().expect("Bind1 detached from graph");
        let f = self.f.as_ref().expect("Bind1 detached from graph");
        let new_value = f(input.observe());
        let new_current = new_value.observe();
        let old_value = self.value.as_ref().expect("Bind1 detached from graph");
        let old_id = old_value.id();
        let same_node = old_value == &new_value;
        let value_changed = self.output.observe() != new_current;

        self.output.state.borrow_mut().value = new_current;
        self.value = Some(new_value.clone());

        let mut callbacks = Vec::new();
        if !same_node {
            callbacks.push(StabilizationCallback::DependenciesUpdated {
                from: vec![old_id],
                to: vec![new_value.id()],
            });
        }
        if value_changed {
            callbacks.push(StabilizationCallback::ValueChanged);
        }
        callbacks
    }

    fn depth(&self) -> i32 {
        self.output.depth()
    }

    fn adjust_depth(&mut self, new_depth: i32) {
        self.output.state.borrow_mut().depth = new_depth;
    }

    fn teardown(&mut self) {
        self.value = None;
        self.input = None;
        self.f = None;
    }
}

/// A node whose upstream dependency can change at runtime. Create with [`Incrementars::bind`].
///
/// Cloning a `Bind1` produces a second handle to the same node.
pub struct Bind1<I, O> {
    pub(crate) inner: Incr<O>,
    pub(crate) marker: PhantomData<fn(I) -> O>,
}

impl<I, O> Clone for Bind1<I, O> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<I, O: Clone + PartialEq> Observable<O> for Bind1<I, O> {
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

impl<I, O> Bind1<I, O> {
    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, O> {
        self.inner.observe_ref()
    }
}

impl<I, O: Clone + PartialEq> IntoInput<O> for Bind1<I, O> {
    fn into_input(self) -> Incr<O> {
        self.inner
    }
}

impl<I, O: Clone + PartialEq> IntoInput<O> for &Bind1<I, O> {
    fn into_input(self) -> Incr<O> {
        self.inner.clone()
    }
}
