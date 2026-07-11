Create an arbitrary precision complex number ([dashu_cmplx::CBig]) with base 2 rounding towards zero.

Each coefficient is a base-2 `FBig` literal (the same grammar as [fbig!]). The literal uses the
algebraic `a+bi` notation, or a `re, im` pair:

```rust
# use dashu_macros::cbig;
let z = cbig!(11+100i);   // 3 + 4i in base 2
let r = cbig!(111);        // purely real (7)
let im = cbig!(10i);       // purely imaginary (2i)
let p = cbig!(11, -100);  // pair form: 3 - 4i
assert_eq!(z, p + cbig!(1000i)); // (3-4i) + 8i = 3+4i
```
