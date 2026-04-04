use super::traits::{Cutoff, InternalNode, Observable, Signal, StabilizationResult};

pub(crate) struct MapNode1<I, O> {
    pub(crate) output: Signal<O>,
    pub(crate) input: Option<Signal<I>>,
    pub(crate) f: Option<Box<dyn Fn(I) -> O>>,
    pub(crate) cutoff: Option<Cutoff<O>>,
}

impl<I: Clone + 'static, O: 'static> InternalNode for MapNode1<I, O> {
    fn stabilize(&mut self) -> StabilizationResult {
        let input = self.input.as_ref().expect("Map1 detached from graph");
        let f = self.f.as_ref().expect("Map1 detached from graph");
        let new_value = f(input.observe());
        let mut output = self.output.state.borrow_mut();
        if let Some(cutoff) = &self.cutoff {
            if cutoff.check(&new_value, &output.value) {
                return StabilizationResult::Unchanged;
            }
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
        self.input = None;
        self.f = None;
        self.cutoff = None;
    }
}
