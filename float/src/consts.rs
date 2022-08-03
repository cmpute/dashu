//! Constants calculation

// TODO: provide following functions
// enum Constant { E, Pi, Tau, Log2, Sqrt2 } (maybe follow std::f64:consts, optionally also support Phi, Euler, ..)
// const_inline(Constant): generate math constants smaller than two limbs, use precomputed values
// const(precision, Constant): generate math constants to desired precision, use precomputed values
// maybe also add utility functions to calculate more constants: mul, inv or div, half, third
// maybe add a struct called InlineRepr with two words and an exponenet, which is the inlined version of Repr, supporting const operations
//     and has a FloatRepr::from_inlined
