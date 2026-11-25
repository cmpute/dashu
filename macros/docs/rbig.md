Create an arbitrary precision rational number ([dashu_ratio::RBig] or [dashu_ratio::Relaxed]).

```rust
# use dashu_macros::rbig;
let a = rbig!(22/7);
let b = rbig!(~-1/13); // use `~` to create a relaxed rational number
let c = rbig!(0x3c/0x5e);
let d = rbig!(~0xff/dd); // the prefix of denomiator can be omitted
let e = rbig!(-2); // denominators can be omitted for integers

// underscores can be used to separate digits
let f = rbig!(107_241/35_291);
```

For an arbitrary base, add `base N`:
```rust
# use dashu_macros::rbig;
let g = rbig!(a3/gp1 base 32);

// it might be necessary to put a underscore to prevent
// Rust from recognizing some digits as prefix or exponent
let h = rbig!(~_100ef/_5ge base 32);
let i = rbig!(_0b102/_0h2 base 32);
let j = rbig!(b102/h2 base 32);
assert_eq!(i, j);

// the denominator can be omitted here as well
let k = rbig!(a3 base 32);
```

The numerator and the denominator must be separated by a `/`, and each of them
takes at most one sign. Anything else is rejected at compile time:
```compile_fail
# use dashu_macros::rbig;
let _ = rbig!(1 - 2); // not an expression: this is neither -1 nor -1/2
```
```compile_fail
# use dashu_macros::rbig;
let _ = rbig!(1 2);
```

For numbers whose the numerator and denominator are small enough (fit in [u32]),
the literal can be assigned to a constant.

```rust
# use dashu_macros::rbig;
use dashu_ratio::{RBig, Relaxed};

const A: RBig = rbig!(-1/2);
const B: Relaxed = rbig!(~3355/15);
```