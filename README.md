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
