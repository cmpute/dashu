Create an arbitrary precision unsigned integer ([dashu_int::UBig])

Usually just pass use a numeric literal. This works for bases 2, 8, 10 or 16 using standard
prefixes:
```rust
# use dashu_macros::ubig;
let a = ubig!(100);
let b = ubig!(0b101);
let c = ubig!(0o202);
let d = ubig!(0x2ff);
let e = ubig!(314159265358979323846264338327950288419716939937);

// underscores can be used to separate digits
let f = ubig!(0x5a4653ca_67376856_5b41f775_d6947d55_cf3813d1);
```

For an arbitrary base, add `base N`:
```rust
# use dashu_macros::ubig;
let g = ubig!(a3gp1 base 32);

// it might be necessary to put a underscore to prevent
// Rust from recognizing some digits as prefix or exponent
let h = ubig!(_100ef base 32);
let i = ubig!(_0b102 base 32);
let j = ubig!(b102 base 32);
assert_eq!(i, j);
```

For numbers that are small enough (fits in a [u32]), the literal can
be assigned to a constant.
```rust
# use dashu_macros::ubig;
use dashu_int::UBig;

const A: UBig = ubig!(123);
const B: UBig = ubig!(0x123);
const C: UBig = ubig!(0xffff_ffff);
```

Please use the [static_ubig!][crate::static_ubig!] macro if you want to create a big static number.