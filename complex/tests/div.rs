//! Division tests (`div`, `inv`, `div_real`): the numerators and the denominator are formed
//! exactly and each part is a single correctly-rounded real division, so `z / z` is exactly `1`
//! and the rounding direction and `Exact`/`Inexact` tag are those of the exact quotient.
// Several tests here need exponents beyond 32 bits and are compiled for 64-bit targets only;
// their helpers and imports are then unused elsewhere.
#![cfg_attr(not(target_pointer_width = "64"), allow(unused_imports, dead_code))]

use dashu_base::Approximation::*;
use dashu_base::{Abs, Inverse};
use dashu_cmplx::{CBig, Context, FBig, FpError};
use dashu_float::round::mode::{Down, HalfAway, HalfEven, Up, Zero};
use dashu_float::round::Round;
use dashu_float::{Context as FloatContext, Repr};
use dashu_int::{IBig, Word};
use proptest::prelude::*;

/// The precisions every `CBig` change is exercised at.
const PRECISIONS: [usize; 4] = [20, 50, 100, 500];

/// `sig · B^exp` at precision `p`.
fn f<R: Round, const B: Word>(sig: impl Into<IBig>, exp: isize, p: usize) -> FBig<R, B> {
    FBig::from_repr(Repr::new(sig.into(), exp), FloatContext::new(p))
}

fn c<R: Round, const B: Word>(re: i128, im: i128, p: usize) -> CBig<R, B> {
    CBig::from_parts(f(re, 0, p), f(im, 0, p))
}

#[test]
fn self_division_is_exactly_one() {
    // z / z = 1 exactly, also when im/re is not representable (1/3, 3/5, 2/7)
    for p in PRECISIONS {
        for (a, b) in [(3, 4), (1, 2), (1, 3), (3, 1), (3, 5), (2, 7), (-7, 2)] {
            let z: CBig<HalfEven, 2> = c(a, b, p);
            let q = Context::<HalfEven>::new(p).div(&z, &z).unwrap();
            assert!(matches!(q, Exact(_)), "({a}+{b}i)/({a}+{b}i), p={p}: {q:?}");
            let q = q.value();
            assert_eq!(q.re(), &Repr::<2>::one(), "p={p}");
            assert!(q.im().is_pos_zero(), "({a}+{b}i)/({a}+{b}i), p={p}: {q:?}");
            assert_eq!(&z / &z, q);

            let z: CBig<HalfAway, 10> = c(a, b, p);
            let q = &z / &z;
            assert_eq!(q.re(), &Repr::<10>::one(), "p={p}");
            assert!(q.im().is_pos_zero(), "decimal ({a}+{b}i)/({a}+{b}i), p={p}: {q:?}");
        }
        // a real multiple divided by its base value
        let q = &c::<HalfAway, 10>(2, 6, p) / &c::<HalfAway, 10>(1, 3, p);
        assert_eq!(q.re(), &Repr::<10>::new(2.into(), 0), "p={p}");
        assert!(q.im().is_pos_zero(), "p={p}: {q:?}");
    }
}

#[test]
fn div_respects_directed_rounding() {
    // i / (1 + 2^k i) = (2^k + i) / (2^2k + 1): the real part is strictly below 2^-k
    for (p, k) in [
        (20usize, 12isize),
        (53, 20),
        (53, 34),
        (100, 60),
        (500, 300),
    ] {
        let z = CBig::<Down, 2>::from_parts(f(0, 0, p), f(1, 0, p));
        let w = CBig::<Down, 2>::from_parts(f(1, 0, p), f(1, k, p));
        let q = Context::<Down>::new(p).div(&z, &w).unwrap();
        assert!(matches!(q, Inexact(..)), "p={p}, k={k}");
        let (re, im) = q.value().into_parts();
        // exact references through the real division
        let den: FBig<Down, 2> = f((IBig::ONE << (2 * k as usize)) + IBig::ONE, 0, 0);
        let fctx = FloatContext::<Down>::new(p);
        let want_re = fctx
            .div(&Repr::new(1.into(), k), den.repr())
            .unwrap()
            .value();
        let want_im = fctx.div(&Repr::one(), den.repr()).unwrap().value();
        assert_eq!(re.repr(), want_re.repr(), "p={p}, k={k}");
        assert_eq!(im.repr(), want_im.repr(), "p={p}, k={k}");
        assert!(re < f::<Down, 2>(1, -k, p), "Down result must stay below 2^-{k}");
    }
}

#[test]
fn zero_numerator_keeps_the_context_precision() {
    for p in PRECISIONS {
        let zero: CBig<HalfEven, 2> = c(0, 0, p);
        let w: CBig<HalfEven, 2> = c(0, 1, p);
        let q = &zero / &w;
        assert!(q.is_zero());
        assert_eq!(q.precision(), p);
        assert!(q.abs().repr().is_pos_zero()); // a transcendental on the quotient works
        let q = Context::<HalfEven>::new(p).div(&zero, &w).unwrap();
        assert!(matches!(q, Exact(_)));
        assert_eq!(q.value().precision(), p);
    }
}

#[test]
fn infinite_operand_is_rejected_before_the_zero_checks() {
    type F = FBig<HalfEven, 2>;
    type C = CBig<HalfEven, 2>;
    let ctx = Context::<HalfEven>::new(10);
    let inf = || F::INFINITY;
    let zero = || C::from_parts(F::ZERO, F::ZERO);
    for z in [
        C::from_parts(inf(), F::ZERO),
        C::from_parts(F::ZERO, inf()),
        C::from_parts(F::NEG_INFINITY, inf()),
    ] {
        assert_eq!(ctx.div(&z, &zero()).unwrap_err(), FpError::InfiniteInput);
        assert_eq!(
            ctx.div(&z, &C::from_parts(F::from(3), F::ZERO))
                .unwrap_err(),
            FpError::InfiniteInput
        );
        assert_eq!(ctx.div(&zero(), &z).unwrap_err(), FpError::InfiniteInput);
        assert_eq!(ctx.inv(&z).unwrap_err(), FpError::InfiniteInput);
    }
    assert_eq!(ctx.div(&zero(), &zero()).unwrap_err(), FpError::Indeterminate);
}

#[test]
#[cfg(target_pointer_width = "64")]
fn reciprocal_near_the_exponent_range_limits() {
    // 1/(2^e·i) = −2^(−e)·i is representable although |z|² is not; `inv` and `1 / z` agree.
    type C = CBig<HalfEven, 2>;
    for e in [
        -(1isize << 62) + 10,
        -(1isize << 62) - 10,
        (1isize << 62) + 10,
    ] {
        let w = C::from_parts(f(0, 0, 53), f(1, e, 53));
        let inv = (&w).inv();
        assert!(inv.re().significand().is_zero(), "e={e}: {inv:?}");
        assert_eq!(inv.im(), &Repr::<2>::new((-1).into(), -e), "e={e}");
        let div = &C::ONE / &w;
        assert_eq!(div.im(), inv.im(), "e={e}");
    }
}

/// A divisor whose parts are thousands of digits apart is still divided exactly: the tag, the
/// value and the rounding direction are those of the exact quotient (the exact numerator and
/// denominator are affordable for the oracle here, though past the width the implementation
/// forms them at).
#[test]
fn wide_exponent_spread_is_correctly_rounded() {
    const E: usize = 20000;
    fn check<R: Round>(p: usize) {
        let big = f::<R, 2>(1, E as isize, 0);
        let unit = f::<R, 2>(1, 0, 0);
        let fctx = FloatContext::<R>::new(p);
        let ctx = Context::<R>::new(p);
        for (u, v) in [
            (unit.clone(), big.clone()),
            (big.clone(), unit.clone()),
            (f::<R, 2>(-3, 0, 0), f::<R, 2>(5, E as isize, 0)),
            (f::<R, 2>(7, E as isize, 0), f::<R, 2>(-1, -(E as isize), 0)),
        ] {
            let den = &u * &u + &v * &v;
            let w = CBig::from_parts(u.clone(), v.clone());
            // 1/w = (u − iv)/(u² + v²)
            let inv = ctx.inv(&w).unwrap();
            assert!(matches!(inv, Inexact(..)), "inv({u:?} + {v:?}i) at {p}: tagged exact");
            let (re, im) = inv.clone().value().into_parts();
            assert_eq!(re, fctx.div(u.repr(), den.repr()).unwrap().value(), "inv re, p={p}");
            assert_eq!(im, fctx.div((-v.clone()).repr(), den.repr()).unwrap().value());
            assert_correct_quotient(&re, &u, &den, p, "inv re");
            assert_correct_quotient(&im, &-v.clone(), &den, p, "inv im");
            assert_eq!(ctx.div(&CBig::ONE, &w).unwrap(), inv);

            // (x + iy)/w with the flags of the two real divisions
            let (x, y) = (f::<R, 2>(11, 3, 0), f::<R, 2>(-13, -2, 0));
            let z = CBig::from_parts(x.clone(), y.clone());
            let q = ctx.div(&z, &w).unwrap();
            let want_re = fctx.div((&x * &u + &y * &v).repr(), den.repr()).unwrap();
            let want_im = fctx.div((&y * &u - &x * &v).repr(), den.repr()).unwrap();
            let flag = |r: &dashu_float::round::Rounded<FBig<R, 2>>| match r {
                Exact(_) => None,
                Inexact(_, rnd) => Some(*rnd),
            };
            match &q {
                Exact(_) => panic!("tagged exact"),
                Inexact(_, flags) => {
                    assert_eq!(Some(flags.0), flag(&want_re), "p={p}");
                    assert_eq!(Some(flags.1), flag(&want_im), "p={p}");
                }
            }
            let (re, im) = q.value().into_parts();
            assert_eq!((re, im), (want_re.value(), want_im.value()), "p={p}");

            // an exact multiple divides exactly: (3 − 2i)·w / w
            let (a, b) = (f::<R, 2>(3, 0, 0), f::<R, 2>(-2, 0, 0));
            let z = CBig::from_parts(&a * &u - &b * &v, &a * &v + &b * &u);
            let q = ctx.div(&z, &w).unwrap();
            assert!(matches!(q, Exact(_)), "exact multiple, p={p}");
            assert_eq!(q.value().into_parts(), (a, b));
        }
    }
    for p in PRECISIONS {
        check::<HalfEven>(p);
        check::<HalfAway>(p);
        check::<Zero>(p);
        check::<Down>(p);
        check::<Up>(p);
    }

    // the reported case: under `Down` the real part of 1/(1 + 2^5000 i) is below 2^-10000
    let w = CBig::<Down, 2>::from_parts(f(1, 0, 0), f(1, 5000, 0));
    let inv = Context::<Down>::new(53).inv(&w).unwrap();
    assert!(matches!(inv, Inexact(..)));
    assert_eq!(inv.value().re(), f::<Down, 2>((1i64 << 53) - 1, -10053, 0).repr());
}

/// The quotient is found without intermediates that leave the exponent range, and at any
/// exponent spread: a part that is in range is returned, and a part that is not saturates on
/// its own.
#[test]
#[cfg(target_pointer_width = "64")]
fn quotient_in_range_with_intermediates_out_of_range() {
    fn check<R: Round>(p: usize) {
        let ctx = Context::<R>::new(p);
        let p2 = |e: isize| f::<R, 2>(1, e, p);
        let zero = || f::<R, 2>(0, 0, p);
        let one = CBig::from_parts(p2(0), zero());

        // 1/(2^e + 2^e i) = 2^-(e+1) − 2^-(e+1) i, exactly, up to the last exponent
        for e in [
            isize::MAX - 3,
            isize::MAX - 2,
            -(isize::MAX - 2),
            (1 << 62) + 5,
        ] {
            let w = CBig::from_parts(p2(e), p2(e));
            let want = CBig::from_parts(p2(-(e + 1)), -p2(-(e + 1)));
            assert_eq!(ctx.inv(&w).unwrap(), Exact(want.clone()), "e={e} p={p}");
            assert_eq!(ctx.div(&one, &w).unwrap(), Exact(want.clone()), "e={e} p={p}");
            assert_eq!((&w).inv(), want);
            assert_eq!(&one / &w, want);
            // w / w and (3w) / w
            assert_eq!(ctx.div(&w, &w).unwrap(), Exact(one.clone()));
            let w3 = CBig::from_parts(f::<R, 2>(3, e - 2, p), f::<R, 2>(3, e - 2, p));
            let w1 = CBig::from_parts(p2(e - 2), p2(e - 2));
            let three = CBig::from_parts(f::<R, 2>(3, 0, p), zero());
            assert_eq!(ctx.div(&w3, &w1).unwrap(), Exact(three));
        }
        let e = isize::MAX - 1;
        let w = CBig::from_parts(p2(e), p2(e));
        let want = CBig::from_parts(p2(-(e + 1)), -p2(-(e + 1)));
        assert_eq!(ctx.inv(&w).unwrap(), Exact(want));

        // 1/(2^E + 2^-E i) = (2^-E − 2^-3E i)/(1 + 2^-4E): each part is just below a power
        // of two in magnitude
        let below = |e: isize, negative: bool| {
            let eps = f::<R, 2>(1, -(p as isize) - 5, 0);
            let near = f::<R, 2>(1, 0, 0) - eps;
            let near = if negative { -near } else { near };
            let (sig, exp) = near.with_precision(p).value().into_repr().into_parts();
            Repr::<2>::new(sig, exp + e)
        };
        for e in [1isize << 40, 1 << 60, 1 << 61] {
            let w = CBig::from_parts(p2(e), p2(-e));
            let inv = ctx.inv(&w).unwrap();
            assert!(matches!(inv, Inexact(..)), "E={e} p={p}: tagged exact");
            assert_eq!(inv.value_ref().re(), &below(-e, false), "E={e} p={p}");
            assert_eq!(inv.value_ref().im(), &below(-3 * e, true), "E={e} p={p}");
            assert_eq!(ctx.div(&one, &w).unwrap(), inv);
        }
        // … and with E = 2^62 the imaginary part alone is below the range
        let e = 1isize << 62;
        let w = CBig::from_parts(p2(e), p2(-e));
        let inv = ctx.inv(&w).unwrap();
        assert!(matches!(inv, Inexact(..)));
        assert_eq!(inv.value_ref().re(), &below(-e, false), "p={p}");
        assert!(inv.value_ref().im().is_neg_zero(), "p={p}");
        assert_eq!((&w).inv().re(), &below(-e, false));
        // the mirrored divisor loses its real part instead
        let w = CBig::from_parts(p2(-e), p2(e));
        let inv = ctx.inv(&w).unwrap().value();
        assert!(inv.re().is_pos_zero(), "p={p}");
        assert_eq!(inv.im(), &below(-e, true), "p={p}");

        // an astronomically wide divisor: (3 − 2i)·w / w is exact, 1/w is not
        let e = 1isize << 50;
        let (u, v) = (f::<R, 2>(1, 0, p), f::<R, 2>(1, e, p));
        let w = CBig::from_parts(u, v);
        // (3 − 2i)(1 + 2^e i) = (3 + 2^(e+1)) + (3·2^e − 2)i needs e bits per part: use the
        // purely real and purely imaginary multiples instead
        let w3 = CBig::from_parts(f::<R, 2>(3, 0, p), f::<R, 2>(3, e, p));
        let three = CBig::from_parts(f::<R, 2>(3, 0, p), zero());
        assert_eq!(ctx.div(&w3, &w).unwrap(), Exact(three));
        let wi = CBig::from_parts(f::<R, 2>(-1, e, p), f::<R, 2>(1, 0, p)); // i·w
        let i = CBig::from_parts(zero(), f::<R, 2>(1, 0, p));
        assert_eq!(ctx.div(&wi, &w).unwrap(), Exact(i));
        let inv = ctx.inv(&w).unwrap();
        assert!(matches!(inv, Inexact(..)));
        assert_eq!(inv.value_ref().re(), &below(-2 * e, false), "p={p}");
        assert_eq!(inv.value_ref().im(), &below(-e, true), "p={p}");
    }
    for p in PRECISIONS {
        check::<HalfEven>(p);
        check::<HalfAway>(p);
        check::<Zero>(p);
        check::<Down>(p);
        check::<Up>(p);
    }
}

/// `|num − q·den| ≤ ½·ulp(q)·den` (nearest) or `0 ≤ ± (num − q·den) < ulp(q)·den` (directed),
/// with all quantities exact (unlimited-precision `FBig` arithmetic is lossless).
fn assert_correct_quotient<R: Round>(
    q: &FBig<R, 2>,
    num: &FBig<R, 2>,
    den: &FBig<R, 2>,
    p: usize,
    what: &str,
) {
    let x = |v: &FBig<R, 2>| FBig::<R, 2>::from_repr(v.repr().clone(), FloatContext::new(0));
    if num.repr().significand().is_zero() {
        assert!(q.repr().significand().is_zero(), "{what}: zero numerator, p={p}");
        return;
    }
    assert!(q.repr().digits() <= p, "{what}: {q:?} has more than {p} digits");
    let q = x(q);
    // one unit in the last place of a full `p`-digit significand
    let ulp = FBig::<R, 2>::from_repr(
        Repr::new(1.into(), q.repr().exponent() + q.repr().digits() as isize - p as isize),
        FloatContext::new(0),
    );
    let err = x(num) - &q * x(den); // den > 0
    let bound = ulp * x(den);
    let name = core::any::type_name::<R>();
    let zero = FBig::<R, 2>::ZERO;
    if name.ends_with("HalfEven") || name.ends_with("HalfAway") {
        assert!(err.clone().abs() + err.clone().abs() <= bound, "{what}: not nearest, p={p}");
    } else if name.ends_with("Down") {
        assert!(err >= zero && err < bound, "{what}: not the floor, p={p}");
    } else if name.ends_with("Up") {
        assert!(err <= zero && -err.clone() < bound, "{what}: not the ceiling, p={p}");
    } else {
        // Zero: truncation — the error has the sign of the quotient
        assert!(err.clone().abs() < bound, "{what}: not truncated, p={p}");
        assert!(err.repr().significand().is_zero() || err.repr().sign() == num.repr().sign());
    }
}

fn operand() -> impl Strategy<Value = (i128, isize)> {
    prop_oneof![
        (any::<i64>().prop_map(i128::from), -70isize..70),
        (-(1i128 << 100)..(1i128 << 100), -70isize..70),
        (-4i128..4, -70isize..70),
    ]
}

fn check_div<R: Round>(ops: [(i128, isize); 4]) {
    let [x, y, u, v] = ops.map(|(sig, exp)| f::<R, 2>(sig, exp, 0));
    if u.repr().significand().is_zero() && v.repr().significand().is_zero() {
        return;
    }
    let den = &u * &u + &v * &v;
    let (re_num, im_num) = (&x * &u + &y * &v, &y * &u - &x * &v);
    for p in PRECISIONS {
        let wide = |re: &FBig<R, 2>, im: &FBig<R, 2>| {
            CBig::from_parts(
                re.clone().with_precision(200).value(),
                im.clone().with_precision(200).value(),
            )
        };
        let (z, w) = (wide(&x, &y), wide(&u, &v));
        let q = Context::<R>::new(p).div(&z, &w).unwrap();
        let exact_tag = matches!(q, Exact(_));
        let (re, im) = q.value().into_parts();
        assert_correct_quotient(&re, &re_num, &den, p, "re");
        assert_correct_quotient(&im, &im_num, &den, p, "im");
        // the tag is `Exact` iff both parts reproduce the numerators exactly
        let x0 = |v: &FBig<R, 2>| FBig::<R, 2>::from_repr(v.repr().clone(), FloatContext::new(0));
        let is_exact = x0(&re) * &den == re_num && x0(&im) * &den == im_num;
        assert_eq!(exact_tag, is_exact, "tag, p={p}");

        // inv(w) = 1/w
        let inv = Context::<R>::new(p).inv(&w).unwrap().value();
        let (re, im) = inv.into_parts();
        assert_correct_quotient(&re, &u, &den, p, "inv re");
        assert_correct_quotient(&im, &-v.clone(), &den, p, "inv im");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn div_is_correctly_rounded(ops in [operand(), operand(), operand(), operand()]) {
        check_div::<HalfEven>(ops);
        check_div::<HalfAway>(ops);
        check_div::<Zero>(ops);
        check_div::<Down>(ops);
        check_div::<Up>(ops);
    }

    #[test]
    fn exact_multiples_divide_exactly(q in [operand(), operand()], w in [operand(), operand()]) {
        // z = q·w exactly, so z / w = q whenever q fits in the precision
        let [a, b] = q.map(|(sig, exp)| f::<HalfEven, 2>(sig, exp, 0));
        let [u, v] = w.map(|(sig, exp)| f::<HalfEven, 2>(sig, exp, 0));
        prop_assume!(!(u.repr().significand().is_zero() && v.repr().significand().is_zero()));
        let z = CBig::from_parts(&a * &u - &b * &v, &a * &v + &b * &u);
        let w = CBig::from_parts(u, v);
        let got = Context::<HalfEven>::new(500).div(&z, &w).unwrap();
        prop_assert!(matches!(got, Exact(_)));
        let (re, im) = got.value().into_parts();
        prop_assert_eq!(re, a);
        prop_assert_eq!(im, b);
    }
}
