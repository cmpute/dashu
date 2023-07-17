use dashu_base::{DivRem, DivRemAssign, Sign::Negative};
use dashu_int::fast_div::ConstDivisor;

mod helper_macros;

#[test]
fn test_value() {
    let ring = ConstDivisor::new(ubig!(100));
    assert_eq!(ring.value(), ubig!(100));

    let ring = ConstDivisor::new(ubig!(10).pow(100));
    assert_eq!(ring.value(), ubig!(10).pow(100));
}

#[test]
fn test_const_div() {
    let divisors = [
        ubig!(1),
        ubig!(2),
        ubig!(3),
        ubig!(127),
        ubig!(12345),
        (ubig!(1) << 32) - ubig!(1),
        (ubig!(1) << 64) - ubig!(1),
        (ubig!(1) << 128) - ubig!(1),
        (ubig!(1) << 256) - ubig!(1),
    ];

    let numbers = [
        ubig!(0),
        ubig!(1),
        ubig!(2),
        ubig!(3),
        ubig!(3).pow(5),
        ubig!(3).pow(10),
        ubig!(3).pow(20),
        ubig!(3).pow(40),
        ubig!(3).pow(80),
        ubig!(3).pow(120),
        ubig!(3).pow(160),
    ];

    for d in &divisors {
        let const_d = &(ConstDivisor::new(d.clone()));
        for n in &numbers {
            assert_eq!(n / d, n / const_d);
            assert_eq!(n / d, n.clone() / const_d);
            assert_eq!(n % d, n % const_d);
            assert_eq!(n % d, n.clone() % const_d);
            assert_eq!(n.div_rem(d), n.div_rem(const_d));
            assert_eq!(n.div_rem(d), n.clone().div_rem(const_d));

            let mut x = n.clone();
            x /= const_d;
            assert_eq!(x, n / d);

            let mut x = n.clone();
            x %= const_d;
            assert_eq!(x, n % d);

            let mut x = n.clone();
            assert_eq!(x.div_rem_assign(const_d), n % d);
            assert_eq!(x, n / d);

            let i = &(Negative * n.clone());
            assert_eq!(i / d, i / const_d);
            assert_eq!(i / d, i.clone() / const_d);
            assert_eq!(i % d, i % const_d);
            assert_eq!(i % d, i.clone() % const_d);
            // assert_eq!(i.div_rem(d), n.div_rem(const_d));
            // assert_eq!(i.div_rem(d), n.clone().div_rem(const_d));

            let mut x = i.clone();
            x /= const_d;
            assert_eq!(x, i / d);

            let mut x = i.clone();
            x %= const_d;
            assert_eq!(x, i % d);

            let mut x = i.clone();
            assert_eq!(x.div_rem_assign(const_d), i % d);
            assert_eq!(x, i / d);
        }
    }
}
