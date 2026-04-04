use super::traits::{Cutoff, InternalNode, Observable, Signal, StabilizationResult};

pub(crate) struct MapNode3<I1, I2, I3, O> {
    pub(crate) output: Signal<O>,
    pub(crate) input1: Option<Signal<I1>>,
    pub(crate) input2: Option<Signal<I2>>,
    pub(crate) input3: Option<Signal<I3>>,
    pub(crate) f: Option<Box<dyn Fn(I1, I2, I3) -> O>>,
    pub(crate) cutoff: Option<Cutoff<O>>,
}

impl<I1: Clone + 'static, I2: Clone + 'static, I3: Clone + 'static, O: 'static> InternalNode
    for MapNode3<I1, I2, I3, O>
{
    fn stabilize(&mut self) -> StabilizationResult {
        let input1 = self.input1.as_ref().expect("Map3 detached from graph");
        let input2 = self.input2.as_ref().expect("Map3 detached from graph");
        let input3 = self.input3.as_ref().expect("Map3 detached from graph");
        let f = self.f.as_ref().expect("Map3 detached from graph");
        let new_value = f(input1.observe(), input2.observe(), input3.observe());
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
        self.input1 = None;
        self.input2 = None;
        self.input3 = None;
        self.f = None;
        self.cutoff = None;
    }
}
