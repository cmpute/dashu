Create an arbitrary precision float number ([dashu_float::DBig]) with base 10 rounding to the nearest.

```rust
# use dashu_macros::dbig;
let a = dbig!(12.001);
let b = dbig!(7.42e-3); // exponent in base 2 can be specified using `Bxx`

// underscores can be used to separate digits
let c = dbig!(3.141_592_653_589_793_238);
```

The generated float has precision determined by length of digits in the input literal.
```rust
# use dashu_macros::dbig;
let a = dbig!(12.001); // 5 decimal digits
assert_eq!(a.precision(), 5);

let b = dbig!(003.1200e-2); // 7 decimal digits
assert_eq!(b.precision(), 7);
assert_eq!(b.digits(), 3); // 312 only has 3 effective digits
```

For numbers whose significands are small enough (fit in a [u32]),
the literal can be assigned to a constant.
```rust
# use dashu_macros::dbig;
use dashu_float::DBig;

const A: DBig = dbig!(-1.201);
const B: DBig = dbig!(1234_5678e-100);
const C: DBig = dbig!(-1e100000);
```