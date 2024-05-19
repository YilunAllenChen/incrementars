pub trait Node {
    fn id(&self) -> usize;
    fn stablize(&mut self);
    fn height(&self) -> usize;
}

pub trait Observable<T> {
    fn observe(&self) -> T;
    fn height(&self) -> usize;
}
