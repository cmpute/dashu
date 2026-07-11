//! Drop-in parity between [`FBig`] and [`CachedFBig`] for the always-on trait surface.
//!
//! Every trait mirrored onto [`CachedFBig`] must behave identically to the inner [`FBig`].
//! Third-party (serde / num-traits / rand / …) traits are intentionally *not* mirrored; the only
//! check for them here is that `.as_fbig()` exposes the inner `FBig` (last test).

use core::str::FromStr;

use dashu_base::{AbsOrd, EstimatedLog2, Sign};
use dashu_float::round::mode::{HalfAway, Zero};
use dashu_float::{CachedFBig, FBig};
use dashu_int::{IBig, UBig};

type F = FBig<HalfAway, 10>;
type C = CachedFBig<HalfAway, 10>;

fn f(s: &str) -> F {
    F::from_str(s).unwrap()
}
fn c(s: &str) -> C {
    C::from_str(s).unwrap()
}

/// Assert a `CachedFBig` equals the matching `FBig` by value.
fn eq(cval: C, expected: &F) {
    assert_eq!(cval.as_fbig(), expected);
}

#[test]
fn ref_operator_parity() {
    // The reference-operand operator variants must compile and match FBig (drop-in guarantee).
    use dashu_base::{DivEuclid, DivRemEuclid, RemEuclid};
    let (fa, fb) = (f("6.25"), f("2.5"));
    let (ca, cb) = (c("6.25"), c("2.5"));

    // Arithmetic on references (all four ownership combinations exercised across the operators).
    eq(&ca + &cb, &(&fa + &fb));
    eq(&ca - cb.clone(), &(&fa - fb.clone()));
    eq(ca.clone() * &cb, &(fa.clone() * &fb));
    eq(&ca / &cb, &(&fa / &fb));
    eq(&ca % &cb, &(&fa % &fb));

    // Neg on a reference.
    eq(-&ca, &(-&fa));

    // Euclid family on references.
    assert_eq!(DivEuclid::div_euclid(&ca, &cb), DivEuclid::div_euclid(&fa, &fb));
    eq(RemEuclid::rem_euclid(&ca, &cb), &RemEuclid::rem_euclid(&fa, &fb));
    let (cq, cr) = DivRemEuclid::div_rem_euclid(&ca, &cb);
    let (fq, fr) = DivRemEuclid::div_rem_euclid(&fa, &fb);
    assert_eq!(cq, fq);
    eq(cr, &fr);
}

#[test]
fn fmt_parity() {
    // Display / LowerExp / UpperExp (base 10)
    for s in &["0", "1.5", "-1.234", "100", "0.001", "999.999"] {
        let fval = f(s);
        let cval = c(s);
        assert_eq!(format!("{}", cval), format!("{}", fval), "Display {s}");
        assert_eq!(format!("{:e}", cval), format!("{:e}", fval), "LowerExp {s}");
        assert_eq!(format!("{:E}", cval), format!("{:E}", fval), "UpperExp {s}");
    }

    // Base-specific formatters: Binary/LowerHex (base 2), Octal (base 8), LowerHex (base 16).
    type F2 = FBig<HalfAway, 2>;
    type C2 = CachedFBig<HalfAway, 2>;
    let (f2, c2) = (F2::from_str("101").unwrap(), C2::from_str("101").unwrap()); // 5
    assert_eq!(format!("{:b}", c2), format!("{:b}", f2), "Binary");
    assert_eq!(format!("{:x}", c2), format!("{:x}", f2), "LowerHex b2");

    type F8 = FBig<HalfAway, 8>;
    type C8 = CachedFBig<HalfAway, 8>;
    let (f8, c8) = (F8::from_str("17").unwrap(), C8::from_str("17").unwrap()); // 15
    assert_eq!(format!("{:o}", c8), format!("{:o}", f8), "Octal");

    type F16 = FBig<HalfAway, 16>;
    type C16 = CachedFBig<HalfAway, 16>;
    let (f16, c16) = (F16::from_str("ff").unwrap(), C16::from_str("ff").unwrap()); // 255
    assert_eq!(format!("{:x}", c16), format!("{:x}", f16), "LowerHex b16");
}

#[test]
fn ordering_parity() {
    let (a, b) = (f("1.5"), f("-2.25"));
    let (ca, cb) = (c("1.5"), c("-2.25"));
    assert_eq!(ca.cmp(&cb), a.cmp(&b));
    assert_eq!(ca.partial_cmp(&cb), a.partial_cmp(&b));

    // AbsOrd: same-base, and vs UBig / IBig (both directions).
    assert_eq!(AbsOrd::abs_cmp(&ca, &cb), AbsOrd::abs_cmp(&a, &b));
    let u = UBig::from(2u32);
    let i = IBig::from(-3);
    assert_eq!(AbsOrd::abs_cmp(&ca, &u), AbsOrd::abs_cmp(&a, &u));
    assert_eq!(AbsOrd::abs_cmp(&u, &cb), AbsOrd::abs_cmp(&u, &b));
    assert_eq!(AbsOrd::abs_cmp(&ca, &i), AbsOrd::abs_cmp(&a, &i));
    assert_eq!(AbsOrd::abs_cmp(&i, &cb), AbsOrd::abs_cmp(&i, &b));
}

#[test]
fn sign_and_log2_parity() {
    for s in &["1.5", "-1.5", "0"] {
        assert_eq!(c(s).sign(), f(s).sign(), "sign {s}");
        assert_eq!(c(s).log2_bounds(), f(s).log2_bounds(), "log2_bounds {s}");
        assert_eq!(c(s).log2_est(), f(s).log2_est(), "log2_est {s}");
    }
}

#[test]
fn fromstr_and_from_parity() {
    // FromStr attaches a fresh cache; the value matches a From<FBig> construction.
    eq(c("1.234"), &f("1.234"));

    // From<primitive> / From<UBig> / From<IBig>.
    eq(C::from(7u8), &F::from(7u8));
    eq(C::from(-9i32), &F::from(-9i32));
    eq(C::from(UBig::from(123u32)), &F::from(UBig::from(123u32)));
    eq(C::from(IBig::from(-456)), &F::from(IBig::from(-456)));

    // TryFrom<f64> (base 2 only).
    type F2 = FBig<HalfAway, 2>;
    type C2 = CachedFBig<HalfAway, 2>;
    let fv = F2::try_from(1.5f64).unwrap();
    let cv = C2::try_from(1.5f64).unwrap();
    assert_eq!(cv.as_fbig(), &fv);
}

#[test]
fn try_into_parity() {
    // TryFrom<CachedFBig> for ints/floats delegates to the inner FBig.
    let cval = c("-12.0");
    let fval = f("-12.0");
    assert_eq!(IBig::try_from(cval.clone()).ok(), IBig::try_from(fval.clone()).ok());
    assert_eq!(i32::try_from(cval.clone()).ok(), i32::try_from(fval.clone()).ok());
    assert_eq!(u32::try_from(cval).ok(), u32::try_from(fval).ok()); // negative → err

    // Base-2 float round-trip.
    type C2 = CachedFBig<HalfAway, 2>;
    let cv = C2::try_from(1.5f64).unwrap();
    assert_eq!(f64::try_from(cv).unwrap(), 1.5f64);
}

#[test]
fn shift_parity() {
    eq(c("1.5") << 2, &(f("1.5") << 2));
    eq(c("1.5") >> 1, &(f("1.5") >> 1));
}

#[test]
fn sign_mul_parity() {
    eq(c("3.0") * Sign::Negative, &f("-3.0"));
    eq(Sign::Negative * c("3.0"), &f("-3.0"));
    let mut m = c("3.0");
    m *= Sign::Negative;
    eq(m, &f("-3.0"));
}

#[test]
fn root_and_euclid_parity() {
    use dashu_base::{CubicRoot, DivEuclid, DivRemEuclid, Inverse, RemEuclid, SquareRoot};

    eq(SquareRoot::sqrt(&c("16.0")), &SquareRoot::sqrt(&f("16.0")));
    eq(CubicRoot::cbrt(&c("8.0")), &CubicRoot::cbrt(&f("8.0")));

    let (cn, cd) = (c("17.0"), c("5.0"));
    let (n, d) = (f("17.0"), f("5.0"));
    assert_eq!(
        DivEuclid::div_euclid(cn.clone(), cd.clone()),
        DivEuclid::div_euclid(n.clone(), d.clone()),
    );
    eq(
        RemEuclid::rem_euclid(cn.clone(), cd.clone()),
        &RemEuclid::rem_euclid(n.clone(), d.clone()),
    );
    let (qf, rf) = DivRemEuclid::div_rem_euclid(n, d);
    let (qc, rc) = DivRemEuclid::div_rem_euclid(cn, cd);
    assert_eq!(qc, qf);
    eq(rc, &rf);

    eq(Inverse::inv(c("4.0")), &Inverse::inv(f("4.0")));
}

#[test]
fn sum_product_parity() {
    let strs = ["1.234", "5.6", "0.001", "-0.5"];
    let fvals: Vec<F> = strs.iter().map(|s| f(s)).collect();
    let cvals: Vec<C> = strs.iter().map(|s| c(s)).collect();

    eq(cvals.iter().sum::<C>(), &fvals.iter().sum::<F>());
    eq(cvals.clone().into_iter().sum::<C>(), &fvals.clone().into_iter().sum::<F>());
    eq(cvals.iter().product::<C>(), &fvals.iter().product::<F>());

    // Empty iterator → additive/multiplicative identity (Default = zero / one with a fresh cache).
    eq(core::iter::empty::<C>().sum::<C>(), &F::ZERO);

    // Headline: 11 × 0.9 at precision 1, Zero mode — the exact sum 9.9 truncates to 9, and the
    // CachedFBig sum (delegating to FBig's correctly-rounded Sum) must match the FBig sum.
    type Z = CachedFBig<Zero, 10>;
    type FZ = FBig<Zero, 10>;
    let zvals: Vec<Z> = (0..11).map(|_| Z::from_parts(9.into(), -1)).collect();
    let zfvals: Vec<FZ> = (0..11).map(|_| FZ::from_parts(9.into(), -1)).collect();
    assert_eq!(zvals.iter().sum::<Z>().as_fbig(), &zfvals.iter().sum::<FZ>());
    assert_eq!(zvals.iter().sum::<Z>().as_fbig(), &FZ::from_parts(9.into(), 0));
}

#[test]
fn as_fbig_accessor() {
    // Third-party traits are reached through as_fbig(); it returns the inner &FBig verbatim.
    let cval = c("1.5");
    let dbg = format!("{:?}", cval.as_fbig());
    assert!(dbg.contains("prec"));
    assert_eq!(cval.as_fbig(), &f("1.5"));
}
