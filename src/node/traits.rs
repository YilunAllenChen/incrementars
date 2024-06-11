pub enum StablizationCallback {
    ValueChanged,
    DependenciesUpdated { from: Vec<usize>, to: Vec<usize> },
}

pub trait Node {
    fn id(&self) -> usize;
    fn stablize(&mut self) -> Vec<StablizationCallback>;
    fn depth(&self) -> i32;
    fn adjust_depth(&mut self, new_depth: i32);
}

pub trait Observable<T> {
    fn id(&self) -> usize;
    fn observe(&self) -> T;
    fn depth(&self) -> i32;
}

impl<T> PartialEq for dyn Observable<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

pub trait MaybeDirty {
    fn id(&self) -> usize;
    fn is_dirty(&self) -> bool;
}
