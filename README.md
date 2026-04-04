# incrementars

![incrementars](https://github.com/YilunAllenChen/incrementars/assets/32376517/3151ae7f-b7c4-436f-a0f5-5595af5bfafb)

Experimental incremental-computing framework for Rust.

## Background

Based on Umut A. Acar's [original paper on self-adjusting computation](https://drive.google.com/file/d/19UcnvDS1_6opK5qZcceuDjHTLmG_9Ovf/view), heavily inspired by Jane Street's [Incremental](https://github.com/janestreet/incremental) OCaml library.

The core idea: declare a computation graph once. When inputs change, only the affected portion of the graph re-runs.

## Comparison with Jane Street's Incremental

| Feature | incrementars | Jane Street Incremental |
|---------|-------------|-------------------------|
| Ordering | Height-based, min-heap | Height-based, array-of-lists |
| Node types | Var, Map1–3, MapN, Bind1 | Var, Map, Bind, Observer, Expert, Freeze, Clock |
| Cutoff | Per-node `Fn(&O, &O) -> bool` | Per-node `('a -> 'a -> bool)` |
| Observer pattern | None — read any node freely | Explicit `Observer` marks needed outputs |
| Node states | dirty / clean | invalid / necessary / stale |
| Cycle detection | None | Detects during height adjustment |
| Thread safety | Single-threaded (`Rc/RefCell`) | Single-threaded |

**What's deliberately different:**

- **No Observer pattern.** Jane Street requires explicit observers because OCaml's GC needs them to know which subgraphs to keep alive. Rust has explicit `remove()` instead, which is more ergonomic and requires no extra API surface.
- **No three-state node model.** The invalid/necessary/stale model exists in Jane Street to handle bind-induced subgraph invalidation before the GC runs. Rust's `remove()` handles this explicitly.
- **Eager initial evaluation.** Derived nodes compute their value immediately on creation. Jane Street requires a stabilize call before the first read. The Rust ownership model makes the eager approach more natural.

## Features

- `Var` — mutable input nodes
- `Map` / `Map2` / `Map3` — transform one, two, or three upstream values
- `MapN` — transform a homogeneous list of upstream values
- `Bind` — dynamic rewiring: the upstream dependency can change at runtime
- `watch` / `unwatch` — post-stabilize callbacks
- `remove` — eagerly tear down a node and all its downstream dependents
- Cutoff optimization — nodes skip downstream propagation when their output is unchanged
- `map_with_cutoff` — supply a custom equality predicate instead of `PartialEq`
- `set_if_changed` — skip marking a `Var` dirty when the value hasn't changed

## Quick Example

```rust
use incrementars::prelude::{Incrementars, Observable};

let mut dag = Incrementars::new();
let length = dag.var(2.0);
let area = dag.map(&length, |x| x * x);

// Derived nodes compute eagerly on creation.
assert_eq!(area.observe(), 4.0);

length.set(3.0);
// Not yet propagated.
assert_eq!(area.observe(), 4.0);

dag.stabilize();
assert_eq!(area.observe(), 9.0);

let height = dag.var(5.0);
let volume = dag.map2(&area, &height, |x, y| x * y);
assert_eq!(volume.observe(), 45.0);

// Only volume recomputes — area is unchanged.
height.set(10.0);
dag.stabilize();
assert_eq!(volume.observe(), 90.0);
```

Pass `&node` to avoid consuming handles — the graph APIs accept both owned and borrowed forms:

```rust
let x = dag.var(2);
let y = dag.map(&x, |v| v + 1);
assert_eq!(y.observe(), 3);
```

Use `observe_ref()` to borrow the current value without cloning:

```rust
let text = dag.map(&x, |v| format!("value={v}").into_bytes());
assert_eq!(text.observe_ref().as_slice(), b"value=2");
```

### Custom cutoff

Supply a domain-specific equality predicate — useful for float epsilon comparisons or types without `PartialEq`:

```rust
let smoothed = dag.map_with_cutoff(
    &sensor,
    |v| v * 2.0,
    |old, new| (old - new).abs() < 0.01,  // suppress tiny changes
);
```

### `set_if_changed`

Skip propagation when a `Var` is set to its current value:

```rust
x.set_if_changed(42); // marks dirty only if current value != 42
```

### Dynamic graphs with `Bind`

`Bind` lets the upstream dependency change at runtime. The graph is rewired automatically and heights are recalculated:

```rust
let picker = dag.var(Side::Left);
let result = dag.bind(&picker, move |side| match side {
    Side::Left  => left.clone().into_input(),
    Side::Right => right.clone().into_input(),
});
picker.set(Side::Right);
dag.stabilize(); // result now tracks `right`
```

## Performance

Per-node stabilization overhead on a linear chain (Apple Silicon, release build):

| Chain length | Raw Rust loop | incrementars | Overhead/node |
|---|---|---|---|
| 100 | 128 ns | 2,900 ns | ~28 ns |
| 1,000 | 1,261 ns | 25,900 ns | ~25 ns |
| 10,000 | 12,534 ns | 251,000 ns | ~24 ns |
| 100,000 | 125,370 ns | 2,646,000 ns | ~25 ns |

The ~25 ns/node overhead comes from `Rc<RefCell>` borrows, heap operations, dirty-flag checks, and cutoff dispatch. For any node whose `f` does meaningful work, this overhead is negligible.
