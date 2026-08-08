Create an arbitrary precision complex number ([dashu_cmplx::CBig]), with base 2 rounding towards zero, as a static reference.

The syntax is the same as [cbig!], but this macro generates a **reference to** an immutable static
`CBig` instance. Unlike [cbig!] — which can be lifted into a `const` only when both coefficients
are small enough to fit in a [`DoubleWord`][dashu_int::DoubleWord] — [`static_cbig!`] works for
arbitrarily large coefficients and in any `const`/`static` context.

The generated `CBig` has **unlimited precision**: remember to set a precision (e.g. via
[`with_precision`][dashu_float::FBig::with_precision] on each part) before operating on it. As
with [`static_fbig!`], its inline representation also takes extra static memory to support
cross-platform definitions.

This macro is available only after Rust 1.64 (it relies on `static` items with const generics).

```rust
# use dashu_macros::static_cbig;
let z: &dashu_cmplx::CBig = static_cbig!(3+4i); // 3 + 4i

// works for large coefficients that `cbig!` can't lift into a `const`
static BIG: &dashu_cmplx::CBig = static_cbig!(
    0xffffffffffffffffffffffffffffffffffffffffffffffff,
    0xffffffffffffffffffffffffffffffffffffffffffffffff
);
```
