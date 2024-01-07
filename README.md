# incrementars
![incrementars](https://github.com/YilunAllenChen/incrementars/assets/32376517/3151ae7f-b7c4-436f-a0f5-5595af5bfafb)

very experimental incremental-computing framework.

# Background

This little project is heavily inspired by Jane Street's [Incremental Computing Library, Incremental](https://github.com/janestreet/incremental).

### What's different (and going to be different)?

- Well first of all it's done in Rust 🦀 instead of OCaml 🐫.
- Only some of the core features are implemented.
    - Var
    - Map
    - Map2 (technically with the three above, you can already construct any arbitrary statically-structured graphs).
    - Bind2 (allows you to add dynamism to graphs).
- Unlike `Incremental` which is fully baked and battle-tested, `Incrementars` is highly experimental, with little to no optimizations applied (yet).
- I think once I get to it, we can actually harness multithreading to speed things up, unlike OCaml where we're locked on one thread at a time.

### What's similar?

- Incremental computation (duh)
- Easy to use interface
- Strongly typed all the way, and Rust safe.

# So uh, what is Incremental Computing?

Incremental computing is a reactive computing paradigm that allows for computations
to be performed incrementally.

### 🤷 Duh, so **what is incremental computing?**

OK fine. I'll tell you.

It's a framework in which, if part of the inputs changes, the necessary, and only the necessary downstream
parts are recomputed, instead of the whole system, allowing for fast and efficient execution of algorithms.

### How does it work?

The computation is modeled with an Directed Acyclic Graph (DAG), where a node can depend on its children. Each node represents some sort of computation, or some values.
Once a node changes, all nodes that depend on it (and their children, and their children's children, ... until we get to the end of it) are recomputed.

Think this pseudocode:
```rust
fn on_change(changed_node) {
    let queue = [changed_node];
    while queue.not_empty() {
        let node = queue.pop();
        node.recompute();
        queue.extend(node.children);
    }
}
```

Pretty cool eh?

# Show me some examples.

Sure! Here's a toy example.

Here we define 3 variables (v1, v2, v3). They're just some inputs to our computation.
And then we define two computation nodes:

- m1 = v1 + v2. It takes in two `Variable` nodes.
- m2 = m1 \* v3. This one is slightly more interesting. It takes in one `Variable` node, and one `Map` node.
- m3 = (m1 + m2) -> to_string. This one is even more interesting. It takes two nodes, and transforms the node into **another type**!

You see that at first, m3 has a value of ( (1 + 1) * 2 + (1 + 1) ).to_string(), which is "6".

However, as we change v1, and then call `stablize` (which is just a fancy name of `fire away!`),
the value of m2 changes to ( (5 + 1) * 2 + (5 + 1) ).to_string(), which is "18". 

```rust
use incrementars::Graph;

fn main() {
    //   v2  v1   ( 1, 1 )  -> ( 1, 5 )
    //    \ /
    // v3  m1     ( 2, 2 )  -> ( 2, 6 )
    //  \ /  \
    //   m2   \   ( 4 )     -> ( 12 )
    //     \  |
    //       m3   ( "6" )   -> ( "18" )

    let mut g = Graph::new();
    let (v1w, v1r) = g.var(1);
    let (_, v2r) = g.var(1);
    let (_, v3r) = g.var(2);
    let (_, m1r) = g.map2(&v1r, &v2r, |a, b| a + b);
    let (_, m2r) = g.map2(&m1r, &v3r, |a, b| a * b);
    let (_, m3r) = g.map2(&m1r, &m2r, |a, b| (a + b).to_string());
    assert_eq!(m3r.borrow().value(), "6".to_string());

    g.stablize();
    v1w.borrow_mut().set(5);

    g.stablize();
    assert_eq!(m3r.borrow().value(), "18".to_string());
}
```

## OK, but why?

You might be asking yourself, how is this useful? Can I just do something like this? Isn't this simpler?

```rust
let m1 = v1 + v2;
let m2 = m1 * v3;
let m3 = (m1 + m2).to_string();
```

And you'd be right. For simple computations like this, you don't need `Incrementars`.
However, think a complex network of thousands or more nodes. In a traditional model, you'd need to recompute
the whole graph every time something changes, because you don't model the topology of the network, and therefore
to guarantee correctness, you have no choice but to redo the whole thing every time.

Whereas in an incremental computing system, the dependency graphs are explicitly modelled, and hence it's safe to refire
only a subset of the system. This not only makes it faster, saves resources, but also makes your system more modular, and hence 
easier to reason about.

For example, think about the case when only v3 changed. In the simple model, you have to recompute everything. Whereas
in `Incrementars`, you know that m1, v1, and v2 are not changed, and hence you can just ignore them. The only computation
you need to redo is the v3 -> m2 -> m3 path, and you'll arrive at the same correct state of the world as the simple model.

Of course, using such a framework will add some space overhead to the system, and for simple calculations, the time overhead is also not negligable.
So the best way to truly harness an incremental computation framework is, like everything else in life, find a good balance point.
