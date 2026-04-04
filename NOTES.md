# Engineering Notes

## Comparison with Jane Street's Incremental (OCaml)

Jane Street's `Incremental` is the production reference for this library. Key design differences:

| Feature | incrementars | Jane Street Incremental |
|---------|-------------|-------------------------|
| Ordering scheme | Height 0→N, min-heap | Height 0→N, min-heap (same after H1 fix) |
| Recompute heap | `BinaryHeap` O(log n) | Array-of-lists indexed by height, O(1) |
| Observer pattern | None — read any node freely | Explicit `Observer` nodes mark needed outputs |
| Node states | dirty / clean | invalid / necessary / stale (three-state) |
| Cutoff | Per-node `Fn(&O, &O) -> bool` (after M1) | Per-node `('a -> 'a -> bool)` |
| Cycle detection | None (infinite loop) | Raises error during height adjustment |
| Thread safety | Single-threaded (`Rc/RefCell`) | Single-threaded (same) |

### What we deliberately don't implement (and why)

- **Observer pattern** — JS needs it for implicit GC-driven cleanup of unneeded subgraphs. Rust has explicit `remove()`, which is more ergonomic and avoids the API overhead of `create_observer` / `disallow_future_use`.
- **Three-state node model** — The dirty/clean two-state model maps cleanly onto Rust's explicit `remove()`. The third "invalid" state exists in JS to handle bind-induced subgraph invalidation before GC runs, which is unnecessary here.
- **Thread safety** — `Rc/RefCell` is correct for single-threaded use. `Arc/Mutex` would add 3–5× overhead on every `observe()` call.

---

## Changes Implemented (2024)

### H1: Height-based ordering — correctness fix

**Problem:** The original scheme used `VAR_DEPTH = 1000` with derived nodes at `min(parents) - 1`. Chains longer than 1000 nodes would get negative depths, and although linear chains still processed in correct order (negative depths still sort monotonically in a max-heap), non-linear graphs (diamonds, merges) with one path longer than 1000 could process nodes out of topological order, producing stale values.

**Fix:** Switched to ascending heights (`VAR_HEIGHT = 0`, derived = `max(parents) + 1`) and a min-heap via `Reverse<i32>`. Now there is no upper bound on graph depth.

**Files:** `src/node/mod.rs` — const rename, all height computations, stabilization heap, bind depth adjustment.

### H3: `Var::set_if_changed`

**Problem:** `set()` always marks dirty, forcing full downstream propagation even when the value didn't change.

**Fix:** Added `set_if_changed(&self, value: T) where T: PartialEq` on `Var`. Checks equality before marking dirty.

**File:** `src/node/var.rs`

### H4: O(1) edge deduplication

**Problem:** `add_edge` used `Vec::contains` — O(n) per call. Building `mapn` with N inputs was O(n²) in total edge insertions.

**Fix:** Added `edge_set: HashSet<(usize, usize)>` to `Incrementars`. `add_edge` now checks the set in O(1).

**File:** `src/node/mod.rs`

### M2: O(1) `unwatch`

**Problem:** `unwatch(id)` iterated all hook slots across all nodes — O(graph_size).

**Fix:** Added `hook_index: HashMap<usize, usize>` mapping hook_id → node_id. `watch()` inserts into the index; `unwatch()` looks up in O(1) and only scans the target node's hooks.

**File:** `src/node/mod.rs`

### M1: Customizable cutoff functions

**Problem:** Cutoff was hardcoded as `PartialEq::eq`. No way to use domain-specific equality (e.g., float epsilon), and output types without `PartialEq` could not use cutoff at all.

**Fix:** Added `cutoff: Option<Box<dyn Fn(&O, &O) -> bool>>` to all `_MapN` structs. Default `map`/`map2`/`map3`/`mapn` builders supply `|a, b| a == b` (still require `O: PartialEq`). New `map_with_cutoff` builder accepts any `O: 'static` with a caller-supplied predicate. Pass `|_, _| false` to disable cutoff entirely.

**Files:** `src/node/map.rs`, `map2.rs`, `map3.rs`, `mapn.rs`, `mod.rs`

---

## Benchmark Results

All benchmarks measure a linear chain of `x + 1` operations, running two full stabilizations per iteration (set 1, stabilize, set 0, stabilize).

### Before changes (baseline, 2024-04-03)

| Chain | `raw_linear` | `linear_stabilize` | Overhead/node |
|-------|--------------|--------------------|---------------|
| 100 | 128 ns | 2,500 ns | ~24 ns |
| 1,000 | 1,261 ns | 22,500 ns | ~21 ns |
| 10,000 | 12,534 ns | 207,500 ns | ~19 ns |
| 100,000 | 125,370 ns | 2,200,000 ns | ~20 ns |

### After changes

| Chain | `raw_linear` | `linear_stabilize` | Overhead/node | Δ vs baseline |
|-------|--------------|--------------------|---------------|---------------|
| 100 | 128 ns | 2,900 ns | ~28 ns | +16% |
| 1,000 | 1,261 ns | 25,900 ns | ~25 ns | +15% |
| 10,000 | 12,534 ns | 251,000 ns | ~24 ns | +21% |
| 100,000 | 125,370 ns | 2,646,000 ns | ~25 ns | +20% |

### Why the regression

The ~4–5 ns/node regression comes almost entirely from **M1 (customizable cutoff)**. The old code inlined `a == b` directly; the new code calls `Box<dyn Fn(&O, &O) -> bool>`, which is a virtual dispatch the compiler cannot inline or devirtualize in general.

The H1, H4, and M2 fixes have negligible per-node runtime cost.

**This is a deliberate tradeoff.** The flexibility to use non-`PartialEq` types and domain-specific equality is worth ~4 ns/node for any real-world computation (where the node's `f` itself will dominate).

If raw performance is critical and all outputs implement `PartialEq`, the old direct-equality path can be recovered by specializing the hot path (see "What to do next" below).

---

## What to Do Next (Prioritized)

### High priority

**H2 — Bind depth adjustment on diamond graphs**
When a bind rewires and two paths from the rewired node converge on the same downstream node, the DFS height propagation sets that node's height from the first path it traverses and then never revisits it when the second path is processed. Fix: propagate using a topological BFS sorted by current height, ensuring all parents are settled before a node's height is updated.
- File: `src/node/mod.rs`, the `adjust_queue` loop in `stabilize()` lines ~480–500

### Medium priority

**Performance: recover inlined cutoff for PartialEq types**
The `Box<dyn Fn>` dispatch costs ~4 ns/node. Two options:
1. Add a separate `enum Cutoff<O> { PartialEq, Custom(Box<dyn Fn(&O, &O) -> bool>) }` and match on it — lets the compiler avoid the vtable for the common case
2. Keep the current generic `map` return a specialized zero-cost type that uses `PartialEq` directly and only the `map_with_cutoff` variant uses dynamic dispatch

**L1 — Array-of-lists recompute heap**
Replace `BinaryHeap<(Reverse<i32>, usize)>` with `Vec<Vec<usize>>` indexed by height. Since real-world graphs have small bounded heights (~70–200), this gives O(1) push and O(max_height) drain vs. O(log n) per operation. Requires H2 to be solid first.
- File: `src/node/mod.rs`

**L2 — Cycle detection**
Add a secondary `Bitmap` tracking "currently being height-adjusted." If the DFS visits a node already in the set, a cycle exists (introduced via `bind`). Currently infinite loops silently.

**L3 — `Bitmap` bounds safety**
`Bitmap::insert` and `Bitmap::contains` silently ignore out-of-bounds values. Replace with `debug_assert!` bounds checks or dynamic growth to catch internal bugs earlier.
- File: `src/node/bitmap.rs`

### Not planned

- Observer pattern (see "deliberately not implemented" above)
- Thread safety (`Arc/Mutex`) — no concrete use case; 3–5× overhead
- `Map4`–`Map9` arities — `MapN` handles this; a proc-macro `map_tuple!` is the right long-term answer
- Async stabilization — inherently synchronous; async wrappers belong outside this crate
