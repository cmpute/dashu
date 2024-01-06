Create an arbitrary precision float number ([dashu_float::FBig]), with base 2 rounding towards zero, as a static reference.

The syntax of this macro is the same as the [fbig!][crate::fbig!], but the macro generates a **reference to** a immutable static `FBig` instance. Due to the limitation of const generics, the generated `FBig` instance will take as much as 4x (static) memory as a normal one, to support cross-platform definitions. Besides, the generated float number will have a unlimited precision. Please remember to set a precision before any operations between two static numbers.

This macro is available only after Rust 1.64.