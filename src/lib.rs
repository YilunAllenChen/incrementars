mod node;

pub mod prelude {
    pub use crate::node::{
        Bind1, Incr, Incrementars, IntoInput, Map1, Map2, Map3, MapN, Observable, Var,
    };
}
