Create an arbitrary precision signed integer ([dashu_int::IBig])

Usually just pass use a numeric literal. This works for bases 2, 8, 10 or 16 using standard
prefixes:
```rust
# use dashu_macros::ibig;
let a = ibig!(-100);
let b = ibig!(0b101);
let c = ibig!(-0o202);
let d = ibig!(0x2ff);
let e = ibig!(314159265358979323846264338327950288419716939937);

// underscores can be used to separate digits
let f = ibig!(-0x5a4653ca_67376856_5b41f775_d6947d55_cf3813d1);
```

For an arbitrary base, add `base N`:
```rust
# use dashu_macros::ibig;
let g = ibig!(-a3gp1 base 32);

// it might be necessary to put a underscore to prevent
// Rust from recognizing some digits as prefix or exponent
let h = ibig!(-_100ef base 32);
let i = ibig!(_0b102 base 32);
let j = ibig!(b102 base 32);
assert_eq!(i, j);
```

For numbers that are small enough (fits in a [u32]), the literal can
be assigned to a constant.
```rust
# use dashu_macros::ibig;
use dashu_int::IBig;

const A: IBig = ibig!(-123);
const B: IBig = ibig!(0x123);
const C: IBig = ibig!(-0xffff_ffff);
```

Please use the [static_ibig!][crate::static_ibig!] macro if you want to create a big static number.