# incrementars

![incrementars](https://github.com/YilunAllenChen/incrementars/assets/32376517/3151ae7f-b7c4-436f-a0f5-5595af5bfafb)

very experimental incremental-computing framework.

# Background

Original paper is from Umut A. Acar, you can [find it here](https://drive.google.com/file/d/19UcnvDS1_6opK5qZcceuDjHTLmG_9Ovf/view).

Heavily inspired by Jane Street's [Incremental Computing Library, Incremental](https://github.com/janestreet/incremental).

### What's different (and going to be different)?

- Only some of the core features are implemented.
  - Var
  - Map
  - Map2
  - Map3
  - MapN
  - Bind (allows you to add dynamism to graphs).
- Post-stabilize hooks are implemented via `watch()`, and subgraphs can be removed with `remove()`.

### What's similar?

- Incremental computation (duh)
- Easy to use interface
- Strongly typed all the way, and Rust safe.
- Blazingly fast!

### A Quick Example

Here's a quick example.

```rust
use incrementars::prelude::{Incrementars, Observable};

pub fn main() {
    let mut dag = Incrementars::new();
    let length = dag.var(2.0);
    let area = dag.map(&length, |x| {
        println!("calculating area");
        x * x
    });

    // derived nodes compute their initial value eagerly when they are created.
    assert_eq!(area.observe(), 4.0);
    length.set(3.0);

    // right after setting, dag isn't stabilized yet.
    assert_eq!(area.observe(), 4.0);

    dag.stabilize();
    assert_eq!(area.observe(), 9.0);

    println!("introducing height...");
    let height = dag.var(5.0);
    let volume = dag.map2(&area, &height, |x, y| {
        println!("calculating volume");
        x * y
    });

    assert_eq!(volume.observe(), 45.0);

    println!("setting height (this shouldn't trigger area calculation!)");
    height.set(10.0);
    dag.stabilize();
    assert_eq!(volume.observe(), 90.0);

    println!("setting length (this should trigger area calculation)");
    length.set(2.0);
    dag.stabilize();
    assert_eq!(volume.observe(), 40.0);
}
```

The graph APIs accept direct handles and references, so the common case is just
passing `&node`:

```rust
use incrementars::prelude::{Incrementars, Observable};

let mut dag = Incrementars::new();
let x = dag.var(2);
let y = dag.map(&x, |value| value + 1);
assert_eq!(y.observe(), 3);
```

If you want to avoid cloning outputs on reads, concrete node handles also expose
`observe_ref()`.

`stabilize()` only propagates pending dirty-input changes. It does not perform the
initial computation for newly-created derived nodes.

## NOTE: What's new in V2

I refactored the original implementation. The original implementation involves passing around two node handles (one
for reads and one for writes), which at times can feel unergonomic / confusing. The new implementation is much more
elegant in that it uses a single node handle for both reads and writes.

Internally, it uses `Rc<RefCell>>` heavily. This is a challenge intrinsic to Rust given how ownerships & borrow checking
work.
