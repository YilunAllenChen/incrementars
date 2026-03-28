use std::{cell::Ref, marker::PhantomData};

use super::traits::{Incr, IntoInput, Node, Observable, StabilizationResult};

pub(crate) struct _MapN<T, O> {
    pub(crate) output: Incr<O>,
    pub(crate) inputs: Option<Vec<Incr<T>>>,
    pub(crate) f: Option<Box<dyn Fn(Vec<T>) -> O>>,
}

impl<T: Clone + 'static, O: PartialEq + 'static> Node for _MapN<T, O> {
    fn stabilize(&mut self) -> StabilizationResult {
        let inputs = self.inputs.as_ref().expect("MapN detached from graph");
        let f = self.f.as_ref().expect("MapN detached from graph");
        let new_value = f(inputs.iter().map(|input| input.observe()).collect());
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
        self.inputs = None;
        self.f = None;
    }
}

/// A node that combines a homogeneous list of upstream values through a function.
/// Create with [`Incrementars::mapn`].
///
/// Cloning a `MapN` produces a second handle to the same node.
pub struct MapN<T, O> {
    pub(crate) inner: Incr<O>,
    pub(crate) marker: PhantomData<fn(T) -> O>,
}

impl<T, O> Clone for MapN<T, O> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<T, O: Clone> Observable<O> for MapN<T, O> {
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

impl<T, O> MapN<T, O> {
    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, O> {
        self.inner.observe_ref()
    }
}

impl<T, O: Clone> IntoInput<O> for MapN<T, O> {
    fn into_input(self) -> Incr<O> {
        self.inner
    }
}

impl<T, O: Clone> IntoInput<O> for &MapN<T, O> {
    fn into_input(self) -> Incr<O> {
        self.inner.clone()
    }
}
