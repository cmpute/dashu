#![warn(unfused_mul_add)]
use dashu_float::{FBig, round::mode::HalfEven};
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;

fn next() -> UBig {
    UBig::ONE
}

fn main() {
    let n = std::hint::black_box(UBig::from(12u8));
    let _ = &n * &n;
    let _ = n.clone() * n.clone() * n.clone();
    let _ = n.pow(2);
    let _ = n.pow(3);
    let a = IBig::from(-12);
    let _ = &a * &a; // Suggestion must mention the unsigned result type of sqr.
    let _ = a.pow(2);
    let r = RBig::from(-12);
    let _ = &r * &r;
    let x = FBig::<HalfEven, 10>::from_parts(14.into(), -1)
        .with_precision(2)
        .value();
    let _ = &x * &x;
    let _ = (&x * &x) * &x; // Help only: one rounding is different from two.
    let y = x.clone();
    let z = -x.clone();
    let _ = (&x * &y) + &z; // Opt-in FMA advice, never a replacement.
    // Not flagged: primitives, different places, side effects, ordinary powers.
    let _ = 12u64 * 12u64;
    let _ = &x * &y;
    let _ = next() * next();
    let _ = n.pow(4);
    let _ = n.sqr();
    struct Other;
    impl Other {
        fn pow(&self, _: u32) -> Self {
            Other
        }
    }
    let _ = Other.pow(2);
}
