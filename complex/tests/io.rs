//! `Display` / `FromStr`: the algebraic `a+bi` text must read back as the value that wrote it.

use core::str::FromStr;
use dashu_base::ParseError;
use dashu_cmplx::CBig;
use dashu_float::round::mode::{Down, HalfAway, HalfEven, Up, Zero};
use dashu_float::round::Round;
use dashu_float::{Context as FloatContext, FBig, Repr};
use dashu_int::{IBig, Word};

fn f<R: Round, const B: Word>(sig: impl Into<IBig>, exp: isize) -> FBig<R, B> {
    let sig = sig.into();
    let repr = Repr::new(sig, exp);
    let digits = repr.digits().max(1);
    FBig::from_repr(repr, FloatContext::new(digits))
}

fn c<R: Round, const B: Word>(re: i64, im: i64) -> CBig<R, B> {
    CBig::from_parts(f(re, 0), f(im, 0))
}

#[test]
fn imaginary_coefficient_with_signed_exponent() {
    type C10 = CBig<HalfAway, 10>;
    type F10 = FBig<HalfAway, 10>;
    for (s, re, im) in [
        ("1+2e3i", "1", "2e3"),
        ("1+2e-3i", "1", "2e-3"),
        ("1+2e+3i", "1", "2e+3"),
        ("1-2e-3i", "1", "-2e-3"),
        ("1e-3+2i", "1e-3", "2"),
        ("1e-3-2e-3i", "1e-3", "-2e-3"),
        ("-1.5e+2+2.5E-3i", "-1.5e+2", "2.5E-3"),
        ("2e-3i", "0", "2e-3"),
        ("1e5i", "0", "1e5"),
        ("1e+5i", "0", "1e+5"),
        ("-1e-5i", "0", "-1e-5"),
        ("0e+0i", "0", "0"),
        ("1@-5+3@+2i", "1@-5", "3@+2"),
    ] {
        let z = C10::from_str(s).unwrap_or_else(|e| panic!("{s:?}: {e:?}"));
        assert_eq!(z.re(), F10::from_str(re).unwrap().repr(), "{s}");
        assert_eq!(z.im(), F10::from_str(im).unwrap().repr(), "{s}");
    }
    type C2 = CBig<HalfAway, 2>;
    type F2 = FBig<HalfAway, 2>;
    for (s, re, im) in [
        ("0x1234p-4+0x5678p8i", "0x1234p-4", "0x5678p8"),
        ("0x1234p-4-0x5678p-8i", "0x1234p-4", "-0x5678p-8"),
        ("0x1p-3i", "0", "0x1p-3"),
        ("1.01b-5-1b+7i", "1.01b-5", "-1b+7"),
    ] {
        let z = C2::from_str(s).unwrap_or_else(|e| panic!("{s:?}: {e:?}"));
        assert_eq!(z.re(), F2::from_str(re).unwrap().repr(), "{s}");
        assert_eq!(z.im(), F2::from_str(im).unwrap().repr(), "{s}");
    }
    // in base 16 `e` is a digit, so the sign after it separates the two terms
    let z = CBig::<HalfAway, 16>::from_str("1e-3i").unwrap();
    assert_eq!(z, c::<HalfAway, 16>(0x1e, -3));

    // still malformed
    for s in [
        "1+2e-i", "1e-+3i", "1+-2i", "1-+2i", "++2i", "-+2i", "1+2", "e-3i", "1+e-3i",
    ] {
        assert!(C10::from_str(s).is_err(), "{s:?} must be rejected");
    }
    assert_eq!(C10::from_str("ii").unwrap_err(), ParseError::InvalidSyntax);
    assert_eq!(C10::from_str("1+2ii").unwrap_err(), ParseError::InvalidSyntax);
    assert_eq!(C10::from_str("i5").unwrap_err(), ParseError::InvalidSyntax);
}

#[test]
fn infinite_parts_round_trip() {
    type C = CBig<HalfEven, 10>;
    type F = FBig<HalfEven, 10>;
    for (re, im) in [
        (F::from(1), F::INFINITY),
        (F::from(1), F::NEG_INFINITY),
        (F::INFINITY, F::INFINITY),
        (F::NEG_INFINITY, F::NEG_INFINITY),
        (F::INFINITY, F::from(2)),
        (F::NEG_INFINITY, F::from(-2)),
        (F::INFINITY, F::ZERO),
        (F::NEG_INFINITY, F::ZERO),
        (F::ZERO, F::INFINITY),
        (F::ZERO, F::NEG_INFINITY),
    ] {
        let z = C::from_parts(re, im);
        let s = z.to_string();
        let back = C::from_str(&s).unwrap_or_else(|e| panic!("{s:?}: {e:?}"));
        assert_eq!(back.re(), z.re(), "{s}");
        assert_eq!(back.im(), z.im(), "{s}");
    }
    assert_eq!(C::from_parts(F::from(1), F::NEG_INFINITY).to_string(), "1-infi");
    assert_eq!(C::from_str("infi").unwrap().im(), F::INFINITY.repr());
    // base 2 reads them too
    let z = CBig::<Zero, 2>::from_str("-inf+infi").unwrap();
    assert!(z.re().is_infinite() && z.im().is_infinite());
    // `inf` must be the whole coefficient
    for s in ["1+inffi", "in+1i", "1+2infi", "infinity"] {
        assert!(C::from_str(s).is_err(), "{s:?} must be rejected");
    }
}

#[test]
fn sign_flag_applies_once() {
    type C = CBig<HalfAway, 10>;
    for (re, im, plain, plus) in [
        (5, 0, "5", "+5"),
        (-5, 0, "-5", "-5"),
        (0, 2, "2i", "+2i"),
        (0, -2, "-2i", "-2i"),
        (0, 1, "i", "+i"),
        (0, -1, "-i", "-i"),
        (1, 2, "1+2i", "+1+2i"),
        (1, 1, "1+i", "+1+i"),
        (-3, -4, "-3-4i", "-3-4i"),
        (-3, 4, "-3+4i", "-3+4i"),
        (3, -1, "3-i", "+3-i"),
    ] {
        let z: C = c(re, im);
        assert_eq!(format!("{}", z), plain);
        assert_eq!(format!("{:+}", z), plus);
        assert_eq!(plain.parse::<C>().unwrap(), z, "{plain}");
        assert_eq!(plus.parse::<C>().unwrap(), z, "{plus}");
    }
}

#[test]
fn precision_flag_rounds_the_signed_coefficient() {
    // The imaginary coefficient is rounded as `FBig`'s `Display` rounds that signed value —
    // toward −∞ under Down, toward +∞ under Up — exactly like the real coefficient.
    fn check<R: Round>() {
        let one = FBig::<R, 10>::from_parts(1.into(), 0);
        for (sig, exp, prec) in [
            (-2i64, -1isize, 0usize),
            (2, -1, 0),
            (-25, -2, 1),
            (15, -2, 1),
            (-999, -3, 2),
        ] {
            let v = FBig::<R, 10>::from_parts(sig.into(), exp);
            let alone = format!("{:.*}", prec, v);
            let sep = if alone.starts_with('-') { "" } else { "+" };
            let one_s = format!("{:.*}", prec, one);
            // as the imaginary part, and as the real part
            let z = CBig::<R, 10>::from_parts(one.clone(), v.clone());
            assert_eq!(format!("{:.*}", prec, z), format!("{one_s}{sep}{alone}i"));
            let w = CBig::<R, 10>::from_parts(v.clone(), one.clone());
            assert_eq!(format!("{:.*}", prec, w), format!("{alone}+i"));
            // pure imaginary
            let w = CBig::<R, 10>::from_parts(FBig::ZERO, v);
            assert_eq!(format!("{:.*}", prec, w), format!("{alone}i"));
        }
    }
    check::<Down>();
    check::<Up>();
    check::<Zero>();
    check::<HalfEven>();
    check::<HalfAway>();
    let z = CBig::<Down, 10>::from_parts(
        FBig::from_parts(1.into(), 0),
        FBig::from_parts((-2).into(), -1),
    );
    assert_eq!(format!("{:.0}", z), "1-1i");
    let z = CBig::<Up, 10>::from_parts(
        FBig::from_parts(1.into(), 0),
        FBig::from_parts((-2).into(), -1),
    );
    assert_eq!(format!("{:.0}", z), "1-0i");
}

#[test]
fn bases_where_i_is_a_digit_round_trip() {
    // In bases above 18 the letter `i` is the digit 18, so the imaginary term carries an
    // explicit `*i` marker and a bare trailing `i` stays a digit.
    fn check<const B: Word>() {
        let b = B as i64;
        for (re, im) in [
            (0, 0),
            (0, 57),
            (0, 3 * b + 18), // imaginary coefficient ends in the digit `i`
            (b + 17, 0),
            (b + 18, 0), // real number ending in the digit `i`
            (1, 18),     // coefficient is the single digit `i`
            (18, 1),
            (0, 1),
            (0, -1),
            (18, -18),
            (-18 * b - 18, 18 * b * b + 18),
        ] {
            let z: CBig<Zero, B> = c(re, im);
            let s = z.to_string();
            let back = s
                .parse::<CBig<Zero, B>>()
                .unwrap_or_else(|e| panic!("base {B} {s:?}: {e:?}"));
            assert_eq!(back, z, "base {B}: {re}+{im}i printed as {s:?}");
            let s = format!("{:+}", z);
            assert_eq!(s.parse::<CBig<Zero, B>>().unwrap(), z, "base {B}: {s:?}");
        }
    }
    check::<19>();
    check::<20>();
    check::<23>();
    check::<24>();
    check::<36>();

    type C20 = CBig<Zero, 20>;
    assert_eq!(c::<Zero, 20>(0, 58).to_string(), "2i*i");
    assert_eq!(c::<Zero, 20>(38, 0).to_string(), "1i");
    assert_eq!(c::<Zero, 20>(1, 18).to_string(), "1+i*i");
    assert_eq!(c::<Zero, 20>(0, 1).to_string(), "1*i");
    assert_eq!(c::<Zero, 20>(2, -1).to_string(), "2-1*i");
    assert_eq!("1i".parse::<C20>().unwrap(), c::<Zero, 20>(38, 0));
    assert_eq!("i".parse::<C20>().unwrap(), c::<Zero, 20>(18, 0));
    assert_eq!("*i".parse::<C20>().unwrap(), c::<Zero, 20>(0, 1));
    assert_eq!("3-*i".parse::<C20>().unwrap(), c::<Zero, 20>(3, -1));
    // bases up to 18 keep the plain marker
    assert_eq!(c::<Zero, 18>(17, 35).to_string(), "h+1hi");
    assert_eq!("h+1hi".parse::<CBig<Zero, 18>>().unwrap(), c::<Zero, 18>(17, 35));
    assert!("1*i".parse::<CBig<Zero, 18>>().is_err());
}
