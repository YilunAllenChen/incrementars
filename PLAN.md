# incrementars — improvement plan

## Goal

A **minimal, ergonomic** Rust library for incremental computation, inspired by Jane Street's
`Incremental`. Core primitives: `Var`, `Map`, `Map2`, `Bind`. No runtime, no macros, no unsafe.

---

## Completed

### Correctness
- **Depth adjustment bidirectional** — bind's depth adjustment now works when switching to a
  shallower target (depth increase), not only deeper ones.
- **Depth propagation bug fixed** — the inner adjustment loop previously always walked the bind
  node's children instead of the currently-adjusted node's children.
- **Cutoff optimization** — `Map1`/`Map2` stabilize now compares new vs. old output value and
  skips downstream propagation if unchanged (`O: PartialEq`).

### Performance
- **`reverse_dependencies` map** — parent lookups during bind depth adjustment are now O(parents)
  instead of O(all edges). Both maps are kept in sync via a shared `add_edge()` helper.

### API / ergonomics
- **`map`/`map2` accept closures** — was `fn(I) -> O` (function pointers only), now
  `impl Fn(I) -> O + 'static` so closures that capture environment work.
- **`bind` no longer double-boxes** — was `Box<impl Fn(...)>`, now `impl Fn(...) + 'static`
  (boxed internally). Callers drop the `Box::new()` wrapper.
- **Lifetime `'a` removed from `Incrementars`** — the bound `'a: 'static` made the parameter
  vacuous. Everything is now simply `'static`.
- **`impl Default for Incrementars`** added.

### Encapsulation
- `_Var`, `_Map1`, `_Map2`, `_Bind1` and their fields are `pub(crate)`.
- `node` fields on public wrapper types (`Var`, `Map1`, etc.) are `pub(crate)`.
- `_X` internal types removed from public `use` exports.
- `print()` debug method removed from the public API entirely.

### Hygiene
- `stablize` → `stabilize`, `StablizationCallback` → `StabilizationCallback` everywhere.
- `VAR_DEPTH` named constant replaces the magic `1_000`.
- `lazy_static` removed; the counter test now uses a captured `Arc<AtomicUsize>`.
- Zero library dependencies (`clap` and `log` removed from `[dependencies]`).
- Perf binary uses constants instead of CLI args so `clap` is no longer a library dep.
- `None => {}` / verbose `match get_mut` replaced with `.entry().or_default()`.
- `iter` benchmark name corrected (was `"expand"`).

---

## Completed (continued)

### Safety & correctness
- **`nodes` switched to `HashMap<usize, ...>`** — was a `Vec` indexed by ID, which works only
  while IDs are sequential and nodes are never removed. HashMap makes the invariant explicit and
  safe against future refactors.
- **Duplicate edges fixed** — `add_edge` now deduplicates before inserting into both
  `dependencies` and `reverse_dependencies`. Prevents incorrect behaviour when the same node
  is passed as both inputs to `map2`.
- **Regression test added** — `test_map2_same_input_twice` covers the duplicate-edge case.

### API
- **`Node` trait made `pub(crate)`** — removed from the public prelude. Users have no reason
  to implement or reference it; `Observable` is the right public abstraction.

### Documentation
- Doc comments on `Incrementars`, `var`, `map`, `map2`, `bind`, `stabilize`.
- Doc comments on `Var`, `Map1`, `Map2`, `Bind1`, `Observable`, and their `as_input`/`set`
  methods.
- Working doc-test in `Incrementars` struct comment (verified by `cargo test`).

---

## Remaining / future work

### Known issues
- **Stack overflow at large node counts** — building graphs with ~150k+ nodes overflows the
  stack. Root cause is likely deep call chains during graph construction or stabilize. Worth
  profiling before fixing.
- **Duplicate edges when `map2` gets the same node as both inputs** — `add_edge` is called
  twice for the same (parent, child) pair. Functionally harmless (`visited` deduplicates the
  queue; `min()` is idempotent) but wasteful. Fix: deduplicate in `add_edge` or at call site.

### Missing features
- **`map3` and beyond** — currently `Map2` is the limit for static fan-in. Could add `map3`,
  or better: a variadic `mapN` taking a `Vec<Box<dyn Observable<T>>>`.
- **`observe` without `Clone`** — the `Observable::observe()` contract returns `T` by value,
  forcing `T: Clone` on all concrete impls. An `observe_ref` returning `&T` would allow
  non-`Clone` types and eliminate copies on hot paths.
- **Hooks / sentinels** — fire a callback when a node's value changes post-stabilize (useful
  for driving UI updates, side effects, etc.).
- **`unsubscribe` / node removal** — currently nodes are never removed. Add reference counting
  on the dependency graph so orphaned subgraphs can be collected.

### Design
- **`nodes` Vec indexed by id** — `self.nodes[id]` works only because `id_counter` starts at 0
  and nodes are never removed. If either invariant breaks, this silently accesses the wrong node.
  A `HashMap<usize, Rc<RefCell<dyn Node>>>` would be safer at a small constant-factor cost.
- **`as_input()` ergonomics** — every call site requires `.as_input()` to produce a
  `Box<ConcreteType>`. Implementing `Into<Box<dyn Observable<T>>>` for each node type would let
  callers write `dag.map(var.into(), ...)`, though this conflicts with the bind pattern that
  needs a cloneable concrete box.
- **Thread safety** — `Rc<RefCell<_>>` is intentionally single-threaded. If multi-threaded
  stabilization is ever desired, this would need `Arc<Mutex<_>>` and a parallel queue.
