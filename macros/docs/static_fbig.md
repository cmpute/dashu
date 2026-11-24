Create an arbitrary precision float number ([dashu_float::FBig]), with base 2 rounding towards zero, as a static reference.

The syntax of this macro is the same as the [fbig!][crate::fbig!], but the macro generates a **reference to** a immutable static `FBig` instance. Due to the limitation of const generics, the generated `FBig` instance will take as much as 4x (static) memory as a normal one, to support cross-platform definitions. Besides, the generated float number will have a unlimited precision. Please remember to set a precision before any operations between two static numbers.

The precision is unlimited for every literal, including the short ones that [fbig!][crate::fbig!] turns into a constant with the precision of the literal:

```rust
# use dashu_macros::{fbig, static_fbig};
use dashu_float::FBig;

static A: &FBig = static_fbig!(0x1234p-4);
assert_eq!(*A, fbig!(0x1234p-4));
assert_eq!(A.precision(), 0);
assert_eq!(fbig!(0x1234p-4).precision(), 16);
```
