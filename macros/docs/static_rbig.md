Create an arbitrary precision rational number ([dashu_ratio::RBig] or [dashu_ratio::Relaxed]) as a static reference.

The syntax of this macro is the same as the [rbig!][crate::rbig!], but the macro generates a **reference to** a immutable static `RBig` or `Relaxed` instance. Due to the limitation of const generics, the generated instance can take as much as 4x (static) memory as a normal one, to support cross-platform definitions.

This macro is available only after Rust 1.64.