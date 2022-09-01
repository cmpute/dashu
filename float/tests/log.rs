
mod helper_macros;

#[test]
fn test_log_decimal() {
    let log3 = dbig!(00000000000000000003).ln();
    assert_eq!(log3, dbig!(10986122886681096914e-19));
    let log30000 = dbig!(00000000000000030000).ln();
    assert_eq!(log30000, dbig!(10308952660644292427e-18));
    let log00003 = dbig!(00000000000000000003e-4).ln();
    assert_eq!(log00003, dbig!(-81117280833080730447e-19));
}

// TODO(next): add more test cases and add test for binary float
