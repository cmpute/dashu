use dashu_int::UBig;

mod helper_macros;

#[test]
fn test_remove() {
    let mut zero = ubig!(0);
    assert_eq!(zero.remove(&ubig!(0)), None);
    assert_eq!(zero.remove(&ubig!(1)), None);

    let mut one = ubig!(1);
    assert_eq!(one.remove(&ubig!(0)), None);
    assert_eq!(one.remove(&ubig!(1)), None);

    for i in 0..32 {
        for b in [ubig!(2), ubig!(3), ubig!(10), ubig!(16)] {
            let mut a = b.clone().pow(i) * 5u8;
            assert_eq!(a.remove(&b), Some(i));
            assert_eq!(a, ubig!(5));
        }
    }
}

/// `remove_word` is the single-word specialization of `remove` — they must agree for every
/// single-word factor.
#[test]
fn test_remove_word() {
    use dashu_int::Word;

    let mut zero = ubig!(0);
    assert_eq!(zero.remove_word(0), None);
    assert_eq!(zero.remove_word(1), None);

    let mut one = ubig!(1);
    assert_eq!(one.remove_word(0), None);
    assert_eq!(one.remove_word(1), None);

    for i in 0..32 {
        for b in [2u32, 3, 10, 16] {
            let b = b as Word;
            let mut a = UBig::from(b).pow(i) * 5u8;
            assert_eq!(a.remove_word(b), Some(i));
            assert_eq!(a, ubig!(5));
        }
    }
}

/// A multi-word factor still falls through to the general `remove` path (not delegated to
/// `remove_word`), and must agree with removing it as the exact cofactor.
#[test]
fn test_remove_multiword_factor() {
    let factor = ubig!(0x10000000000000000); // 2^64: multi-word on every Word width
    assert!(factor.as_words().len() > 1);

    let mut a = factor.clone().pow(3) * 5u8;
    assert_eq!(a.remove(&factor), Some(3));
    assert_eq!(a, ubig!(5));
}
