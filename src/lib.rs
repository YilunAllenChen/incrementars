mod node;
mod examples;
pub use node::graph::Graph;
pub use node::traits::*;

pub mod prelude {
    pub use super::node::traits::*;
    pub use super::Graph;
}
