# incrementars

![incrementars](https://github.com/YilunAllenChen/incrementars/assets/32376517/3151ae7f-b7c4-436f-a0f5-5595af5bfafb)

very experimental incremental-computing framework.

# Background

This little project is heavily inspired by Jane Street's [Incremental Computing Library, Incremental](https://github.com/janestreet/incremental).

The original paper is from Umut A. Acar, you can [find it here](https://drive.google.com/file/d/19UcnvDS1_6opK5qZcceuDjHTLmG_9Ovf/view).

### What's different (and going to be different)?

- Well first of all it's done in Rust 🦀 instead of OCaml 🐫.
- Only some of the core features are implemented.
  - Var
  - Map
  - Map2 (technically with the three above, you can already construct any arbitrary statically-structured graphs).
  - Bind (allows you to add dynamism to graphs).
- Unlike `Incremental` which is fully baked and battle-tested, `Incrementars` is highly experimental, with little to no optimizations applied (yet).

### What's similar?

- Incremental computation (duh)
- Easy to use interface
- Strongly typed all the way, and Rust safe.

### A Quick Example

Here's a quick example.

```rust
use incrementars::prelude::{Incrementars, Observable};

pub fn main() {
    let mut dag = Incrementars::new();
    let length = dag.var(2.0);
    let area = dag.map(length.as_input(), |x| {
        println!("calculating area");
        x * x
    });

    // on initial stabalization, area is calculated to be 4.
    assert_eq!(area.observe(), 4.0);
    length.set(3.0);

    // right after setting, dag isn't stablized yet.
    assert_eq!(area.observe(), 4.0);

    dag.stablize();
    assert_eq!(area.observe(), 9.0);

    println!("introducing height...");
    let height = dag.var(5.0);
    let volume = dag.map2(area.as_input(), height.as_input(), |x, y| {
        println!("calculating volume");
        x * y
    });

    assert_eq!(volume.observe(), 45.0);

    println!("setting height (this shouldn't trigger area calculation!)");
    height.set(10.0);
    dag.stablize();
    assert_eq!(volume.observe(), 90.0);

    println!("setting length (this should trigger area calculation)");
    length.set(2.0);
    dag.stablize();
    assert_eq!(volume.observe(), 40.0);
}
```

## NOTE: What's new in V2

I refactored the original implementation. The original implementation involves passing around two node handles (one
for reads and one for writes), which at times can feel unergonomic / confusing. The new implementation is much more
elegant in that it uses a single node handle for both reads and writes.

Internally, it uses `Rc<RefCell>>` heavily. This is a challenge intrinsic to Rust given how ownerships & borrow checking
work.
