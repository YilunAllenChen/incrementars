use std::ops::Deref;

use incrementars::{map1, peek, var};

pub fn main() {
    let var = var(0);
    let mut map = map1(var.clone(), |x| x + 1);
    for _ in 0..100000 {
        map = map1(map, |x| x + 1)
    }
    println!("{}", peek(map));

    var.deref().borrow_mut().set(0);



}
