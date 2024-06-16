pub mod prelude {}

type NodeID = usize;

pub trait Node {
    fn id(&self);
    fn depth(&self);
}

pub trait Observable<T> {
    fn id(&self);
    fn observe(&self) -> T;
}

pub struct Var<T> {
    id: usize,
    value: T,
}

pub struct Map<'a, I: 'a, O> {
    id: usize,
    fun: &'a dyn Fn(I) -> O,
}
