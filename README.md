# incrementars

![incrementars](https://github.com/YilunAllenChen/incrementars/assets/32376517/3151ae7f-b7c4-436f-a0f5-5595af5bfafb)

Experimental incremental-computing framework for Rust.

## Background

Based on Umut A. Acar's [original paper on self-adjusting computation](https://drive.google.com/file/d/19UcnvDS1_6opK5qZcceuDjHTLmG_9Ovf/view), heavily inspired by Jane Street's [Incremental](https://github.com/janestreet/incremental) OCaml library.

The core idea: declare a computation graph once. When inputs change, only the affected portion of the graph re-runs.

## Comparison with Jane Street's Incremental

| Feature | incrementars | Jane Street Incremental |
|---------|-------------|-------------------------|
| Ordering | Height-based, bucket queue | Height-based, array-of-lists |
| Node types | Var, Map1–3, MapN, Bind1 | Var, Map, Bind, Observer, Expert, Freeze, Clock |
| Cutoff | Per-node `Fn(&O, &O) -> bool` | Per-node `('a -> 'a -> bool)` |
| Observer pattern | None — read any node freely | Explicit `Observer` marks needed outputs |
| Node states | dirty / clean | invalid / necessary / stale |
| Cycle detection | Panics on height overflow | Raises error during height adjustment |
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

Benchmarks on Apple Silicon (2021 M1 Max), release build. Graph is built once; each iteration calls `stabilize()` once on a pre-dirtied graph. Per-node overhead = (stabilize − raw equivalent) / N.

### Realistic mixed graph (100 layers × 100 nodes = 10,000 nodes)

Mix: 50% `map`, 25% `map2`, 12.5% `map3`, 12.5% `bind`. All inputs dirtied each iteration; full graph recomputes. Simple arithmetic (`x+1`, `a+b`, `(a+b+c)/3`).

| Nodes | Raw equivalent | Stabilize | Overhead/node |
|-------|----------------|-----------|---------------|
| 10,000 | ~7 µs | 233 µs | **~23 ns** |

With 100 nodes per height level the bucket queue processes each level in a tight loop with no resize — good cache behavior and O(1) per push.

### Linear chain (`x + 1` repeated N times)

Each node sits at a unique height; the bucket queue allocates and scans N distinct slots.

| Chain | Raw loop | Stabilize | Overhead/node |
|-------|----------|-----------|---------------|
| 100 | 66 ns | 4,586 ns | ~45 ns |
| 1,000 | 701 ns | 42,102 ns | ~41 ns |
| 10,000 | 6,991 ns | 429,120 ns | ~42 ns |
| 100,000 | 69,318 ns | 4,872,100 ns | ~48 ns |

### Fanout & join graphs

| Graph | Stabilize |
|-------|-----------|
| Fanout: 1 var → 50,000 maps (height 1) | 653 µs |
| Join: 10,000 vars → 1 mapn | 124 µs |
