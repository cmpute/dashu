//! Fixed regression cases for the complex logarithm: the rounding of `±π` at a negative zero
//! and correct rounding of `ln|z|` next to the unit circle.

mod helpers;

use dashu_cmplx::round::mode::{Away, Down, HalfEven, Up, Zero};
use dashu_cmplx::{CBig, Context};
use helpers::{assert_rounds_from, cb, fb, retag_c, zero};

const PRECISIONS: [usize; 4] = [20, 50, 100, 500];

/// `log(−0 ± 0i) = −∞ ± iπ`, with `±π` rounded in the mode's own direction: `−π` under `Down`
/// is the neighbour *below* `−π` (not the negation of `π` rounded down).
#[test]
fn log_of_negative_zero_rounds_pi_in_the_mode_direction() {
    // π = 804.2477…/256
    macro_rules! check {
        ($mode:ty, $pos:expr, $neg:expr) => {{
            let ctx = Context::<$mode>::new(10);
            let l = ctx
                .log(&CBig::<$mode, 2>::from_parts(zero(true), zero(false)), None)
                .unwrap()
                .value();
            assert!(l.re().is_infinite());
            assert_eq!(l.im(), fb::<$mode, 2>($pos, -8).repr(), "{} +pi", stringify!($mode));
            let l = ctx
                .log(&CBig::<$mode, 2>::from_parts(zero(true), zero(true)), None)
                .unwrap()
                .value();
            assert_eq!(l.im(), fb::<$mode, 2>($neg, -8).repr(), "{} -pi", stringify!($mode));
        }};
    }
    check!(Down, 804, -805);
    check!(Up, 805, -804);
    check!(Zero, 804, -804);
    check!(Away, 805, -805);
    check!(HalfEven, 804, -804);

    // and it agrees with the nonzero neighbour −1 ∓ εi, at every precision
    for p in PRECISIONS {
        macro_rules! neighbour {
            ($mode:ty) => {{
                let ctx = Context::<$mode>::new(p);
                for im_neg in [false, true] {
                    let at_zero = ctx
                        .log(&CBig::<$mode, 2>::from_parts(zero(true), zero(im_neg)), None)
                        .unwrap()
                        .value();
                    let eps = if im_neg { -1 } else { 1 };
                    let near = ctx
                        .log(&cb::<$mode, 2>((-1, 0), (eps, -(4 * p as isize))), None)
                        .unwrap()
                        .value();
                    // arg(−1 ± εi) = ±(π − ε): it rounds like ±π except toward zero from
                    // exactly-below, which is the same neighbour for a non-representable π
                    assert_eq!(at_zero.im(), near.im(), "{} p={p}", stringify!($mode));
                }
            }};
        }
        neighbour!(Down);
        neighbour!(Up);
        neighbour!(Zero);
        neighbour!(Away);
    }
}

/// `Re log z = ln|z|` when `|z|` is within a hair of 1: the modulus rounds to 1 (or to
/// `1 + ulp`) at any moderate working precision, and its logarithm is then pure rounding
/// noise unless the driver keeps refining.
#[test]
fn log_near_unit_circle_is_correctly_rounded() {
    // (precision, re, im, expected real part) — expected values from an exact ln(1+t) series
    for (p, re, im, want) in [
        (33usize, (1i64, -15isize), (1i64, 0isize), (2147483647i64, -62isize)),
        (33, (29, -20), (1, 0), (7054819325, -64)),
        (34, (1, 0), (81, -22), (13759414269, -66)),
        (31, (1, 0), (232471777, -56), (1610610697, -88)),
        (33, (1, -15), (1, 1), (5954088945, -33)),
        (33, (1, -2), (1, 0), (8332184577, -38)),
    ] {
        let z: CBig<HalfEven, 2> = cb(re, im);
        let l = Context::<HalfEven>::new(p).log(&z, None).unwrap().value();
        assert_eq!(
            l.re(),
            fb::<HalfEven, 2>(want.0, want.1).repr(),
            "log({re:?} + i{im:?}) at p={p}"
        );
    }

    // the same shape across the precision sweep and the directed modes, against a much higher
    // precision
    macro_rules! check {
        ($mode:ty) => {{
            for p in PRECISIONS {
                let ctx = Context::<$mode>::new(p);
                let hi = Context::<HalfEven>::new(3 * p + 200);
                for z in [
                    cb::<$mode, 2>((1, 0), (1, -(p as isize) / 2 - 3)),
                    cb((1, -(p as isize) - 7), (-1, 0)),
                    cb((-1, 0), (3, -(p as isize))),
                ] {
                    let got = ctx.log(&z, None).unwrap().value();
                    let want = hi.log(&retag_c(&z), None).unwrap().value();
                    assert_rounds_from(&got, &want, p, &format!("{} log p={p}", stringify!($mode)));
                }
            }
        }};
    }
    check!(HalfEven);
    check!(Down);
    check!(Up);

    // base 10 has exact points on the unit circle: |0.6 + 0.8i| = 1, so the real part is an
    // exact zero
    let z: CBig<HalfEven, 10> = cb((6, -1), (8, -1));
    let l = Context::<HalfEven>::new(30).log(&z, None).unwrap().value();
    assert!(l.re().is_pos_zero());
}
