//! Magnitude comparison (`AbsOrd`): exact for every pair of values, whatever the exponents.
// Several tests here need exponents beyond 32 bits and are compiled for 64-bit targets only;
// their helpers and imports are then unused elsewhere.
#![cfg_attr(not(target_pointer_width = "64"), allow(unused_imports, dead_code))]

use core::cmp::Ordering;
use dashu_base::AbsOrd;
use dashu_cmplx::CBig;
use dashu_float::round::mode::{HalfAway, Zero};
use dashu_float::{Context as FloatContext, FBig, Repr};
use dashu_int::IBig;

type F = FBig<Zero, 2>;
type C = CBig<Zero, 2>;

fn f(sig: i64, exp: isize) -> F {
    F::from_repr(Repr::new(IBig::from(sig), exp), FloatContext::new(0))
}

fn c(re: (i64, isize), im: (i64, isize)) -> C {
    C::from_parts(f(re.0, re.1), f(im.0, im.1))
}

/// Both orders of a comparison.
fn assert_abs_cmp(a: &C, b: &C, want: Ordering) {
    assert_eq!(a.abs_cmp(b), want, "{a:?} against {b:?}");
    assert_eq!(b.abs_cmp(a), want.reverse(), "{b:?} against {a:?}");
}

#[test]
fn abs_cmp_matches_the_exact_squared_moduli() {
    let parts = [
        (0, 0),
        (1, 0),
        (-1, 0),
        (3, 0),
        (-4, 0),
        (5, 0),
        (5, -3),
        (-7, 40),
        (1, -40),
        (3, 41),
        (-25, 37),
    ];
    let mut values = Vec::new();
    for re in parts {
        for im in parts {
            values.push(c(re, im));
        }
    }
    for a in &values {
        for b in &values {
            // unlimited-precision arithmetic is exact
            let (are, aim) = a.clone().into_parts();
            let (bre, bim) = b.clone().into_parts();
            let want = (&are * &are + &aim * &aim).cmp(&(&bre * &bre + &bim * &bim));
            assert_eq!(a.abs_cmp(b), want, "{a:?} against {b:?}");
        }
    }
    // |3 + 4i| = |5| = |−4 − 3i|
    assert_abs_cmp(&c((3, 0), (4, 0)), &c((5, 0), (0, 0)), Ordering::Equal);
    assert_abs_cmp(&c((3, 0), (4, 0)), &c((-4, 0), (-3, 0)), Ordering::Equal);
}

#[test]
#[cfg(target_pointer_width = "64")]
fn abs_cmp_with_parts_far_apart() {
    // The comparison does not align the two parts of a value: 10^E + i against 10^E + 2i is
    // decided by the small parts, for any E.
    type D = CBig<HalfAway, 10>;
    type DF = FBig<HalfAway, 10>;
    for e in [1_000isize, 100_000_000, 1 << 40, 1 << 61] {
        let big = DF::from_parts(1.into(), e);
        let a = D::from_parts(big.clone(), DF::from(1));
        let b = D::from_parts(big.clone(), DF::from(2));
        assert_eq!(a.abs_cmp(&b), Ordering::Less, "E={e}");
        assert_eq!(b.abs_cmp(&a), Ordering::Greater, "E={e}");
        assert_eq!(a.abs_cmp(&a.conj()), Ordering::Equal, "E={e}");
        // swapping the parts does not change the magnitude
        let swapped = D::from_parts(DF::from(-1), big.clone());
        assert_eq!(a.abs_cmp(&swapped), Ordering::Equal, "E={e}");
        assert_eq!(b.abs_cmp(&swapped), Ordering::Greater, "E={e}");
        // a far smaller part still breaks the tie
        let tiny = DF::from_parts(1.into(), -e);
        let c = D::from_parts(big.clone(), tiny);
        assert_eq!(c.abs_cmp(&D::from(big.clone())), Ordering::Greater, "E={e}");
        assert_eq!(c.abs_cmp(&a), Ordering::Less, "E={e}");
    }

    for e in [1 << 50, 1isize << 61] {
        assert_abs_cmp(&c((1, e), (1, 0)), &c((1, e), (2, 0)), Ordering::Less);
        assert_abs_cmp(&c((1, e), (1, -e)), &c((1, e), (0, 0)), Ordering::Greater);
        assert_abs_cmp(&c((1, e), (3, 0)), &c((-3, 0), (-1, e)), Ordering::Equal);
    }
}

#[test]
fn abs_cmp_with_squares_beyond_the_exponent_range() {
    // |z|² leaves the exponent range, |z| does not
    for k in [
        isize::MAX / 2 + 10,
        isize::MAX - 3,
        -(isize::MAX / 2) - 10,
        isize::MIN + 100,
    ] {
        assert_abs_cmp(&c((1, k), (0, 0)), &c((1, k + 1), (0, 0)), Ordering::Less);
        assert_abs_cmp(&c((1, k), (0, 0)), &c((0, 0), (-1, k)), Ordering::Equal);
        assert_abs_cmp(&c((3, k), (4, k)), &c((5, k), (0, 0)), Ordering::Equal);
        assert_abs_cmp(&c((3, k), (4, k)), &c((5, k), (1, k - 60)), Ordering::Less);
        assert_abs_cmp(&c((1, k), (1, k)), &c((1, k), (-1, k)), Ordering::Equal);
        assert_abs_cmp(&c((1, k), (1, k)), &c((1, k + 1), (0, 0)), Ordering::Less);
        // against an ordinary value and a zero
        let ordinary = if k > 0 {
            Ordering::Greater
        } else {
            Ordering::Less
        };
        assert_abs_cmp(&c((1, k), (0, 0)), &c((1, 0), (1, 0)), ordinary);
        assert_abs_cmp(&c((1, k), (1, k)), &c((0, 0), (0, 0)), Ordering::Greater);
    }
    // the two ends of the range against each other
    assert_abs_cmp(
        &c((1, isize::MAX - 3), (0, 0)),
        &c((0, 0), (1, isize::MIN + 100)),
        Ordering::Greater,
    );
}

#[test]
fn abs_cmp_with_infinite_parts() {
    let inf = C::from_parts(F::INFINITY, f(1, 0));
    let neg_inf = C::from_parts(f(1, 0), F::NEG_INFINITY);
    let finite = c((1, isize::MAX - 3), (1, isize::MAX - 3));
    assert_abs_cmp(&inf, &finite, Ordering::Greater);
    assert_abs_cmp(&neg_inf, &finite, Ordering::Greater);
    assert_abs_cmp(&inf, &neg_inf, Ordering::Equal);
    assert_abs_cmp(&inf, &c((0, 0), (0, 0)), Ordering::Greater);
}
