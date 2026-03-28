use std::ops::Deref;
use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use super::traits::{IntoInput, Node, Observable, StabilizationCallback};

pub(crate) struct _Bind1<I, O> {
    pub(crate) id: usize,
    pub(crate) depth: i32,
    pub(crate) current: O,
    pub(crate) value: Option<Box<dyn Observable<O>>>,
    pub(crate) input: Option<Box<dyn Observable<I>>>,
    pub(crate) f: Option<Box<dyn Fn(I) -> Box<dyn Observable<O>>>>,
}

impl<I: 'static, O: Clone + PartialEq + 'static> Node for _Bind1<I, O> {
    fn id(&self) -> usize {
        self.id
    }

    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        let input = self.input.as_ref().expect("Bind1 detached from graph");
        let f = self.f.as_ref().expect("Bind1 detached from graph");
        let new_value = f(input.observe());
        let new_current = new_value.observe();
        let old_value = self.value.as_ref().expect("Bind1 detached from graph");
        let old_id = old_value.id();
        let same_node = old_value == &new_value;
        let value_changed = self.current != new_current;

        let new_id = new_value.id();
        self.current = new_current;
        self.value = Some(new_value);

        let mut callbacks = Vec::new();
        if !same_node {
            callbacks.push(StabilizationCallback::DependenciesUpdated {
                from: vec![old_id],
                to: vec![new_id],
            });
        }
        if value_changed {
            callbacks.push(StabilizationCallback::ValueChanged);
        }
        callbacks
    }
    fn depth(&self) -> i32 {
        self.depth
    }

    fn adjust_depth(&mut self, new_depth: i32) {
        self.depth = new_depth;
    }
    fn teardown(&mut self) {
        self.value = None;
        self.input = None;
        self.f = None;
    }
}

/// A node whose upstream dependency can change at runtime. Create with [`Incrementars::bind`].
///
/// When the `input` node changes, `f` is called to determine which node to read from next.
/// If the result points to a different node, the dependency graph is rewired automatically
/// and depths are recalculated.
///
/// Cloning a `Bind1` produces a second handle to the same node.
pub struct Bind1<I, O> {
    pub(crate) node: Rc<RefCell<_Bind1<I, O>>>,
}

impl<I, O: Clone + PartialEq> Observable<O> for Bind1<I, O> {
    fn id(&self) -> usize {
        self.node.deref().borrow().id
    }
    fn observe(&self) -> O {
        self.node.deref().borrow().current.clone()
    }
    fn depth(&self) -> i32 {
        self.node.deref().borrow().depth
    }
}

impl<I, O> Clone for Bind1<I, O> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
        }
    }
}

impl<I, O: Clone + PartialEq + 'static> Bind1<I, O> {
    /// Returns a boxed clone of this handle, suitable for passing to
    /// [`Incrementars::map`], [`Incrementars::map2`], [`Incrementars::map3`],
    /// [`Incrementars::mapn`], or [`Incrementars::bind`].
    pub fn as_input(&self) -> Box<Bind1<I, O>> {
        Box::new(self.clone())
    }

    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, O> {
        Ref::map(self.node.deref().borrow(), |internal| &internal.current)
    }
}

impl<I: 'static, O: Clone + PartialEq + 'static> IntoInput<O> for Bind1<I, O> {
    fn into_input(self) -> Box<dyn Observable<O>> {
        Box::new(self)
    }
}

impl<I: 'static, O: Clone + PartialEq + 'static> IntoInput<O> for Box<Bind1<I, O>> {
    fn into_input(self) -> Box<dyn Observable<O>> {
        self
    }
}
