use super::traits::{InternalNode, Observable, Signal, StabilizationResult};

pub(crate) struct BindNode<I, O> {
    pub(crate) output: Signal<O>,
    pub(crate) value: Option<Signal<O>>,
    pub(crate) input: Option<Signal<I>>,
    pub(crate) f: Option<Box<dyn Fn(I) -> Signal<O>>>,
}

impl<I: Clone + 'static, O: Clone + PartialEq + 'static> InternalNode for BindNode<I, O> {
    fn stabilize(&mut self) -> StabilizationResult {
        let input = self.input.as_ref().expect("Bind detached from graph");
        let f = self.f.as_ref().expect("Bind detached from graph");
        let new_value = f(input.observe());
        let new_current = new_value.observe();
        let old_value = self.value.as_ref().expect("Bind detached from graph");
        let old_id = old_value.id();
        let same_node = old_value == &new_value;
        let value_changed = self.output.observe() != new_current;

        self.output.state.borrow_mut().value = new_current;
        self.value = Some(new_value.clone());

        if !same_node {
            return StabilizationResult::Rebound {
                from: old_id,
                to: new_value.id(),
                value_changed,
            };
        }
        if value_changed {
            StabilizationResult::Changed
        } else {
            StabilizationResult::Unchanged
        }
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
