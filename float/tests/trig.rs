use dashu_float::ops::Abs;
use dashu_float::{round, DBig, FBig, Repr};

#[test]
fn test_pi() {
    let pi = DBig::pi(10);
    assert_eq!(pi.to_string(), "3.141592654");

    let pi20 = DBig::pi(20);
    assert_eq!(pi20.to_string(), "3.1415926535897932385");

    let pi100 = DBig::pi(100);
    assert_eq!(pi100.to_string(), "3.141592653589793238462643383279502884197169399375105820974944592307816406286208998628034825342117068");

    let pi500 = DBig::pi(500);
    assert_eq!(pi500.to_string(), "3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679821480865132823066470938446095505822317253594081284811174502841027019385211055596446229489549303819644288109756659334461284756482337867831652712019091456485669234603486104543266482133936072602491412737245870066063155881748815209209628292540917153643678925903600113305305488204665213841469519415116094330572703657595919530921861173819326117931051185480744623799627495673518857527248912279381830119491");

    let pi_bin = FBig::<round::mode::HalfAway, 2>::pi(100);
    assert_eq!(pi_bin.to_string(), "11.00100100001111110110101010001000100001011010001100001000110100110001001100011001100010100010111");
}

#[test]
fn test_sin_cos() {
    let x = DBig::ZERO.with_precision(30).value();
    let (s, c) = x.sin_cos();
    assert_eq!(s, DBig::ZERO);
    assert_eq!(c, DBig::ONE.with_precision(30).value());

    let pi = DBig::pi(30);
    let (s, c) = pi.sin_cos();
    assert!(s.abs() < DBig::from_parts(1.into(), -29));
    let neg_one = -DBig::ONE.with_precision(30).value();
    assert!((c - neg_one).abs() < DBig::from_parts(1.into(), -29));
}

#[test]
fn test_tan() {
    let x = DBig::ZERO.with_precision(30).value();
    assert_eq!(x.tan(), DBig::ZERO);

    let pi = DBig::pi(30);
    let pi4: DBig = pi / 4;
    let tan_pi4 = pi4.tan();
    assert!((tan_pi4 - DBig::ONE).abs() < DBig::from_parts(1.into(), -29));
}

#[test]
fn test_atan() {
    let x = DBig::ZERO.with_precision(30).value();
    assert_eq!(x.atan(), DBig::ZERO);

    let one = DBig::ONE.with_precision(30).value();
    let pi = DBig::pi(30);
    let pi4: DBig = pi / 4;
    let atan_one = one.atan();
    assert!((atan_one - pi4).abs() < DBig::from_parts(1.into(), -29));
}

#[test]
fn test_asin_acos() {
    let x = DBig::ZERO.with_precision(30).value();
    assert_eq!(x.asin(), DBig::ZERO);

    let pi = DBig::pi(30);
    let half_pi: DBig = &pi / 2;
    assert!((x.acos() - half_pi).abs() < DBig::from_parts(1.into(), -29));

    let half = DBig::from_parts(5.into(), -1).with_precision(30).value();
    let asin_half = half.asin();
    // asin(0.5) = pi/6
    let pi6: DBig = &pi / 6;
    assert!((asin_half - pi6).abs() < DBig::from_parts(1.into(), -29));
}

#[test]
#[should_panic]
fn test_asin_out_of_domain_panics() {
    // asin(|x| > 1) is out of domain; the FBig convenience layer panics.
    let two = DBig::from_parts(2.into(), 0).with_precision(10).value();
    let _ = two.asin();
}

#[test]
fn test_atan2() {
    let zero = DBig::ZERO.with_precision(30).value();
    let one = DBig::ONE.with_precision(30).value();
    let neg_one = -one.clone();
    let pi = DBig::pi(30);

    // atan2(0, 1) = 0
    assert_eq!(zero.atan2(&one), zero);

    // atan2(1, 0) = pi/2
    let half_pi: DBig = &pi / 2;
    assert!((one.atan2(&zero) - half_pi.clone()).abs() < DBig::from_parts(1.into(), -29));

    // atan2(0, -1) = pi
    assert!((zero.atan2(&neg_one) - &pi).abs() < DBig::from_parts(1.into(), -29));

    // atan2(-1, 0) = -pi/2
    let m_half_pi: DBig = -half_pi;
    assert!((neg_one.atan2(&zero) - m_half_pi).abs() < DBig::from_parts(1.into(), -29));
}

#[test]
#[should_panic]
fn test_atan2_zero_zero_panics() {
    // atan2(0, 0) is indeterminate; the FBig convenience layer panics.
    let z0 = DBig::ZERO.with_precision(10).value();
    let _ = z0.atan2(&z0);
}

#[test]
fn test_atan2_infinities() {
    let x = DBig::ZERO.with_precision(30).value();
    let ctx = x.context();
    let inf = Repr::infinity();
    let neg_inf = Repr::neg_infinity();
    let pi = ctx.pi::<10>(None).value();
    let pi_4 = &pi / 4;
    let pi_3_4 = &pi * 3 / 4;

    // atan2(+inf, +inf) = pi/4
    let res: DBig = ctx.atan2(&inf, &inf, None).unwrap().value();
    let diff: DBig = res - &pi_4;
    assert!(diff.abs() < DBig::from_parts(1.into(), -29));

    // atan2(+inf, -inf) = 3pi/4
    let res: DBig = ctx.atan2(&inf, &neg_inf, None).unwrap().value();
    let diff: DBig = res - &pi_3_4;
    assert!(diff.abs() < DBig::from_parts(1.into(), -29));

    // atan2(-inf, +inf) = -pi/4
    let res: DBig = ctx.atan2(&neg_inf, &inf, None).unwrap().value();
    let diff: DBig = res + &pi_4;
    assert!(diff.abs() < DBig::from_parts(1.into(), -29));

    // atan2(-inf, -inf) = -3pi/4
    let res: DBig = ctx.atan2(&neg_inf, &neg_inf, None).unwrap().value();
    let diff: DBig = res + &pi_3_4;
    assert!(diff.abs() < DBig::from_parts(1.into(), -29));
}
