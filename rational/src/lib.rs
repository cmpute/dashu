mod add;
mod cmp;
mod convert;
mod div;
mod error;
mod fmt;
mod helper_macros;
mod mul;
mod parse;
mod rbig;
mod repr;
mod sign;
mod simplify;

pub use rbig::{RBig, Relaxed};

// TODO: support "nearest", "nearest_ub" and "nearest_lb" to find the closest rational number,
//       given a limit on the denominator, (see https://math.stackexchange.com/q/2438510/815652)
// TODO: also support "from_f32_simplified", "from_f64_simplified", (see https://stackoverflow.com/q/66980340/5960776)
// TODO: support `is_human_readable` option if we support serde (see https://github.com/rust-num/num-rational/issues/90)
