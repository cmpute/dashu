use dashu_base::{Abs, AbsOrd, EstimatedLog2};
use dashu_float::DBig;
use dashu_int::IBig;

fn main() {
    let a = std::hint::black_box(IBig::from(-12));
    let b = std::hint::black_box(IBig::from(3));
    let _ = a.clone().abs().abs_cmp(&b);
    let _ = a.clone().abs().log2_bounds();
    let _ = a.clone().abs().cmp(&b.clone().abs());
    let _ = a.clone().abs() < b.clone().abs();
    let x = DBig::from(-12);
    let y = DBig::from(3);
    let _ = x.clone().abs().abs_cmp(&y);
    // Not flagged: the right-hand side may be negative, or abs is actually needed.
    let _ = a.clone().abs() < b;
    let _ = x.abs() < y;
    let _ = a.clone().abs();
    let _ = (-1.0f64).abs() < 2.0;
}
