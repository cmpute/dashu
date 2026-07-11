Create a static reference to an arbitrary precision complex number ([dashu_cmplx::CBig]).

This is the static variant of [cbig!], requiring Rust 1.64+ (it relies on `static` items with
const generics). See [cbig!] for the literal grammar.

```rust
# use dashu_macros::static_cbig;
let z: &dashu_cmplx::CBig = static_cbig!(11+100i); // 3 + 4i
```
