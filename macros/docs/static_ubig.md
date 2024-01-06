Create an arbitrary precision unsigned integer ([dashu_int::UBig]) as a static reference.

The syntax of this macro is the same as the [ubig!][crate::ubig!], but the macro generates a **reference to** a immutable static `UBig` instance. Due to the limitation of const generics, the generated `UBig` instance will take as much as 4x (static) memory as a normal one, to support cross-platform definitions.

This macro is available only after Rust 1.64.