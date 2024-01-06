# incrementars
very experimental incremental-computing framework.

# What is Incremental Computing?
Incremental computing is a reactive computing paradigm that allows for computations
to be performed incrementally.

### Duh, so **what is incremental computing?**
OK fine. I'll tell you.

It's a framework in which, if part of the inputs changes, the necessary, and only the necessary downstream
parts are recomputed, instead of the whole system. 


### How does it work?
Think a directed acyclic graph, where a node can depend on its children. Each node represents some sort of computation.
Once a node changes, all nodes that depend on it (and their children, and their children's children, until we get to the end of it) are recomputed.


# Show me some examples.
Here's a toy example.

Here we define 3 variables (v1, v2, v3). They're just some inputs to our computation.
And then we define two computation nodes:
- m1 = v1 + v2. It takes in two `Variable` nodes.
- m2 = m1 * v3. This one is slightly more interesting. It takes in one `Variable` node, and one `Map` node.

You see that at first m2 has a value of (2 + 3) * 5 = 25.

However, as we change v1, and then call `stablize` (which is just a fancy name of `fire away!`),
the value of m2 changes to (4 + 3) * 5 = 35.

```rust
use incrementars::Graph;

fn main() {
    let mut g = Graph::new();
    let v1 = g.var(2);
    let v2 = g.var(3);
    let v3 = g.var(5);
    let m1 = g.map2(&v1, &v2, |a, b| a + b);
    let m2 = g.map2(&m1, &v3, |a, b| a * b);
    assert_eq!(m2.borrow().value(), 25);

    v1.borrow_mut().set(4);
    g.stablize();
    assert_eq!(m2.borrow().value(), 35);
}
```

## OK, but why?
You might be asking yourself, how is this useful? Can I just do something like this? Isn't this simpler?
```rust
let output = (v1 + v2) * v3;
```

And you'd be right. For simple computations like this, you don't need `Incrementars`.
However, think a complex network of thousands or more nodes. In a traditional model, you'd need to recompute
the whole graph every time something changes, because you don't model the topology of the network, and therefore
have to recompute the whole thing every time.

Whereas in an incremental computing system, the dependency graphs are explicitly modelled, and hence it's safe to refire 
only a subset of the system. This not only makes it faster, saves resources, but also makes your system easier to reason
about.
