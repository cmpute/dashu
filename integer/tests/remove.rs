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
            assert_eq!(a, 5);
        }
    }
}
