mod helper_macros;

#[test]
fn test_add() {
    assert_eq!(rbig!(1) + rbig!(1), rbig!(2));
    assert_eq!(rbig!(1 / 2) + rbig!(1 / 2), rbig!(1));
    assert_eq!(rbig!(1 / 2) + rbig!(-1 / 2), rbig!(0));
}
