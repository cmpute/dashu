use std::str::FromStr;

use crate::number::Float;

/// `precision` bits of π, computed as `6 · atan(1 / sqrt(3))` from primitive
/// ops only — no library `pi()` builtin — so every [`Float`] implementation
/// runs the same sqrt → div → atan → mul sequence. The benchmark then measures
/// raw per-op throughput rather than algorithm choice (e.g. Chudnovsky).
pub(crate) fn calculate<T: Float>(precision: u32) -> String {
    let inv_sqrt3 = T::from_int(1, precision) / T::from_int(3, precision).sqrt();
    (inv_sqrt3.atan() * T::from_int(6, precision)).to_string()
}

/// Compare two `pi` renderings by value, allowing a few ULP of slack.
///
/// `dashu::Real` rounds toward zero while astro-float / rug round to nearest,
/// so the last bit legitimately differs by up to ~1.5 ULP — a raw string
/// comparison would reject equal values. dashu also strips trailing significand
/// zeros while the others do not. So this parses each rendering back to a
/// `dashu::Real` (`dashu::Real`/`AstroFloat` render binary, `rug::Float`
/// decimal — either is accepted) and compares numerically.
pub(crate) fn within_tolerance(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let parse = |s: &str| -> Option<dashu::Real> {
        dashu::Real::from_str(s).ok().or_else(|| {
            dashu::Decimal::from_str(s)
                .ok()
                .map(|d| d.to_binary().value())
        })
    };
    let (Some(x), Some(y)) = (parse(a), parse(b)) else {
        return false;
    };
    if x == y {
        return true;
    }
    // Use the larger ulp as the yardstick.
    let ulp = x.ulp().max(y.ulp());
    let diff = if x >= y { x - y } else { y - x };
    diff <= ulp * dashu::Real::from(8u32)
}
