pub trait Node {
    fn id(&self) -> usize;
    fn stablize(&mut self);
    fn depth(&self) -> i32;
}

pub trait Observable<T> {
    fn id(&self) -> usize;
    fn observe(&self) -> T;
    fn depth(&self) -> i32;
}

pub trait MaybeDirty {
    fn id(&self) -> usize;
    fn is_dirty(&self) -> bool;
}
