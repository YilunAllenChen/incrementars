use super::traits::{Cutoff, InternalNode, Observable, Signal, StabilizationResult};

pub(crate) struct MapNodeN<T, O> {
    pub(crate) output: Signal<O>,
    pub(crate) inputs: Option<Vec<Signal<T>>>,
    pub(crate) f: Option<Box<dyn Fn(Vec<T>) -> O>>,
    pub(crate) cutoff: Option<Cutoff<O>>,
}

impl<T: Clone + 'static, O: 'static> InternalNode for MapNodeN<T, O> {
    fn stabilize(&mut self) -> StabilizationResult {
        let inputs = self.inputs.as_ref().expect("MapN detached from graph");
        let f = self.f.as_ref().expect("MapN detached from graph");
        let new_value = f(inputs.iter().map(|input| input.observe()).collect());
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
        self.inputs = None;
        self.f = None;
        self.cutoff = None;
    }
}
