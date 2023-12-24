Create an arbitrary precision signed integer ([dashu_int::IBig]) as a static reference.

The syntax of this macro is the same as the [ibig!][crate::ibig!], but the macro generates a **reference to** a immutable static `IBig` instance. Due to the limitation of const generics, the generated `IBig` instance can take as much as 4x (static) memory as a normal one, to support cross-platform definitions.