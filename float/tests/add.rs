mod helper_macros;

#[test]
fn test_add() {
    let binary_cases = [
        (fbig!(0), fbig!(0), fbig!(0)),
        (fbig!(0), fbig!(1), fbig!(1)),
        (fbig!(0x1), fbig!(0x100), fbig!(0x101)),
        (fbig!(0x00001p8), fbig!(0x00001p-8), fbig!(0x10001p-8)),
        (fbig!(0x123p2), fbig!(-0x123p2), fbig!(0))
    ];

    for (a, b, c) in &binary_cases {
        assert_eq!(a + b, *c);
        // assert_eq!(a.clone() + b, *c);
        // assert_eq!(a + b.clone(), *c);
        assert_eq!(a.clone() + b.clone(), *c);
    }
    
    let decimal_cases = [
        (dbig!(0), dbig!(0), dbig!(0)),
        (dbig!(0), dbig!(1), dbig!(1)),
        (dbig!(1), dbig!(100), dbig!(101)),
        (dbig!(00001e2), dbig!(00001e-2), dbig!(10001e-2)),
        (dbig!(123e2), dbig!(-123e2), dbig!(0))
    ];

    for (a, b, c) in &decimal_cases {
        assert_eq!(a + b, *c);
        // assert_eq!(a.clone() + b, *c);
        // assert_eq!(a + b.clone(), *c);
        assert_eq!(a.clone() + b.clone(), *c);
    }
}
