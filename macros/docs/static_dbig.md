Create an arbitrary precision float number ([dashu_float::DBig]), with base 10 rounding to the nearest, as a static reference.

The syntax of this macro is the same as the [dbig!][crate::dbig!], but the macro generates a **reference to** a immutable static `DBig` instance. Due to the limitation of const generics, the generated `DBig` instance will take as much as 4x (static) memory as a normal one, to support cross-platform definitions. Besides, the generated float number will have a unlimited precision. Please remember to set a precision before any operations between two static numbers.

The precision is unlimited for every literal, including the short ones that [dbig!][crate::dbig!] turns into a constant with the precision of the literal:

```rust
# use dashu_macros::{dbig, static_dbig};
use dashu_float::DBig;

static A: &DBig = static_dbig!(12.34);
assert_eq!(*A, dbig!(12.34));
assert_eq!(A.precision(), 0);
assert_eq!(dbig!(12.34).precision(), 4);
```
