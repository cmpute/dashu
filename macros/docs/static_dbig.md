Create an arbitrary precision float number ([dashu_float::DBig]), with base 10 rounding to the nearest, as a static reference.

The syntax of this macro is the same as the [dbig!][crate::dbig!], but the macro generates a **reference to** a immutable static `DBig` instance. Due to the limitation of const generics, the generated `DBig` instance will take as much as 4x (static) memory as a normal one, to support cross-platform definitions. Besides, the generated float number will have a unlimited precision. Please remember to set a precision before any operations between two static numbers.

This macro is available only after Rust 1.64.