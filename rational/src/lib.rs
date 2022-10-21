mod add;
mod cmp;
mod error;
mod fmt;
mod mul;
mod rbig;
mod repr;
mod sign;

pub use rbig::RBig;

// TODO: support "nearest", "nearest_ub" and "nearest_lb" to find the closest rational number,
//       given a limit on the denominator, (see https://math.stackexchange.com/q/2438510/815652)
// TODO: add from_f32_const, from_f64_const, where the denominator is limited in DoubleWord range (no allocation)
// TODO: support `is_human_readable` option if we support serde (see https://github.com/rust-num/num-rational/issues/90)
