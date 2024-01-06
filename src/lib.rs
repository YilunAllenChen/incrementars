use std::rc::Rc;

pub type NodeId = usize;

pub trait Node<T> {
    fn value(&self) -> T;
    fn stablize(&mut self);
}

#[derive(Copy, Clone)]
pub struct Var<T>
where
    T: Copy + Clone,
{
    pub value: T,
}

impl<T> Node<T> for Var<T>
where
    T: Copy,
{
    fn stablize(&mut self) {}
    fn value(&self) -> T {
        self.value
    }
}

pub struct Map2<T, U, V>
where
    T: Clone,
    U: Clone,
    V: Clone,
{
    pub value: T,
    pub parents: (Rc<dyn Node<U>>, Rc<dyn Node<V>>),
    pub func: fn(U, V) -> T,
}

impl<T, U, V> Node<T> for Map2<T, U, V>
where
    T: Clone,
    U: Clone,
    V: Clone,
{
    fn stablize(&mut self) {
        self.value = (self.func)(self.parents.0.value(), self.parents.1.value());
    }
    fn value(&self) -> T {
        self.value.clone()
    }
}

impl<T, U, V> Map2<T, U, V>
where
    T: Clone,
    U: Clone,
    V: Clone,
{
    pub fn from(
        n1: Rc<impl Node<U> + 'static>,
        n2: Rc<impl Node<V> + 'static>,
        func: fn(U, V) -> T,
    ) -> Self {
        Map2 {
            value: (func)(n1.value(), n2.value()),
            parents: (n1, n2),
            func,
        }
    }
}

pub struct Map3<T, U, V, W>
where
    T: Clone,
    U: Clone,
    V: Clone,
    W: Clone,
{
    pub value: T,
    pub parents: (Rc<dyn Node<U>>, Rc<dyn Node<V>>, Rc<dyn Node<W>>),
    pub func: fn(U, V, W) -> T,
}

impl<T, U, V, W> Node<T> for Map3<T, U, V, W>
where
    T: Clone,
    U: Clone,
    V: Clone,
    W: Clone,
{
    fn stablize(&mut self) {
        self.value = (self.func)(
            self.parents.0.value(),
            self.parents.1.value(),
            self.parents.2.value(),
        );
    }
    fn value(&self) -> T {
        self.value.clone()
    }
}

impl<T, U, V, W> Map3<T, U, V, W>
where
    T: Clone,
    U: Clone,
    V: Clone,
    W: Clone,
{
    pub fn from(
        n1: Rc<impl Node<U> + 'static>,
        n2: Rc<impl Node<V> + 'static>,
        n3: Rc<impl Node<W> + 'static>,
        func: fn(U, V, W) -> T,
    ) -> Self {
        Map3 {
            value: (func)(n1.value(), n2.value(), n3.value()),
            parents: (n1, n2, n3),
            func,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_stablize() {
        let mut v1 = Rc::new(Var { value: 1 });
        let v2 = Rc::new(Var { value: 5 });
        let m1 = Rc::new(Map2::from(v1.clone(), v2, |a, b| a * b));
        let v3 = Rc::new(Var { value: 2 });
        let v4 = Rc::new(Var { value: 10 });

        let mut map3 = Map3::from(m1, v3, v4, |a, b, c| a + b + c);
        map3.stablize();

        assert_eq!(map3.value, 17);

        v1.value = 3;
        map3.stablize();
        assert_eq!(map3.value, 27);
    }
}
