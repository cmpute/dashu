mod helper_macros;
use dashu_int::{IBig, UBig};
use postcard::{from_bytes, to_allocvec};
use serde_json::{from_str, to_string};
use serde_test::{assert_de_tokens, assert_tokens, Configure, Token};

#[test]
fn test_ubig_serde() {
    assert_tokens(&ubig!(0).compact(), &[Token::Bytes(&[])]);
    assert_tokens(&ubig!(0).readable(), &[Token::BorrowedStr("0")]);
    assert_de_tokens(&ubig!(0).readable(), &[Token::BorrowedStr("0")]);
    assert_tokens(&ubig!(17).compact(), &[Token::Bytes(&[17])]);
    assert_tokens(&ubig!(17).readable(), &[Token::BorrowedStr("17")]);
    assert_de_tokens(&ubig!(17).compact(), &[Token::Bytes(&[17])]);
    assert_de_tokens(&ubig!(17).readable(), &[Token::BorrowedStr("17")]);
    assert_tokens(
        &ubig!(0x123451234567890abcdef).compact(),
        &[Token::Bytes(&[
            0xef, 0xcd, 0xab, 0x90, 0x78, 0x56, 0x34, 0x12, 0x45, 0x23, 0x1,
        ])],
    );
    assert_tokens(
        &ubig!(0x123451234567890abcdef).readable(),
        &[Token::BorrowedStr("1375482783624620011146735")],
    );
    assert_de_tokens(
        &ubig!(0x123451234567890abcdef).readable(),
        &[Token::BorrowedStr("0x123451234567890abcdef")],
    );
}

#[test]
fn test_ibig_serde() {
    assert_tokens(&ibig!(0).compact(), &[Token::Bytes(&[])]);
    assert_tokens(&ibig!(0).readable(), &[Token::BorrowedStr("0")]);
    assert_de_tokens(&ibig!(0).readable(), &[Token::BorrowedStr("0")]);
    assert_tokens(&ibig!(17).compact(), &[Token::Bytes(&[17, 0])]);
    assert_tokens(&ibig!(17).readable(), &[Token::BorrowedStr("17")]);
    assert_de_tokens(&ibig!(17).readable(), &[Token::BorrowedStr("0x11")]);
    assert_tokens(&ibig!(-17).compact(), &[Token::Bytes(&[17])]);
    assert_tokens(&ibig!(-17).readable(), &[Token::BorrowedStr("-17")]);
    assert_de_tokens(&ibig!(-17).readable(), &[Token::BorrowedStr("-0x11")]);

    // test padding
    assert_tokens(
        &ibig!(0x123451234567890).compact(),
        &[Token::Bytes(&[
            0x90, 0x78, 0x56, 0x34, 0x12, 0x45, 0x23, 0x1,
        ])],
    );
    assert_tokens(
        &ibig!(-0x123451234567890).compact(),
        &[Token::Bytes(&[
            0x90, 0x78, 0x56, 0x34, 0x12, 0x45, 0x23, 0x1, 0x0,
        ])],
    );
    assert_tokens(
        &ibig!(0x123451234567890abcdef).compact(),
        &[Token::Bytes(&[
            0xef, 0xcd, 0xab, 0x90, 0x78, 0x56, 0x34, 0x12, 0x45, 0x23, 0x1, 0x0,
        ])],
    );
    assert_tokens(
        &ibig!(-0x123451234567890abcdef).compact(),
        &[Token::Bytes(&[
            0xef, 0xcd, 0xab, 0x90, 0x78, 0x56, 0x34, 0x12, 0x45, 0x23, 0x1,
        ])],
    );
}

#[test]
fn test_ubig_round_trip() {
    let test_numbers = [
        ubig!(0),
        ubig!(1),
        ubig!(12),
        ubig!(12345),
        ubig!(1234567890),
        ubig!(0x123456789012345678901234567890123456789),
    ];
    for int in &test_numbers {
        // test binary serialization
        let output = to_allocvec(int).unwrap();
        let parsed: UBig = from_bytes(&output).unwrap();
        assert_eq!(&parsed, int);

        // test string serialization
        let output = to_string(int).unwrap();
        let parsed: UBig = from_str(&output).unwrap();
        assert_eq!(&parsed, int);
    }
}

#[test]
fn test_ibig_round_trip() {
    let test_numbers = [
        ibig!(0),
        ibig!(1),
        ibig!(-1),
        ibig!(12),
        ibig!(-123),
        ibig!(12345),
        ibig!(-12345678),
        ibig!(1234567890),
        ibig!(-0x12345678901234567890123456789),
        ibig!(-0x123456789012345678901234567890123456789),
    ];
    for int in &test_numbers {
        // test binary serialization
        let output = to_allocvec(int).unwrap();
        let parsed: IBig = from_bytes(&output).unwrap();
        assert_eq!(&parsed, int);

        // test string serialization
        let output = to_string(int).unwrap();
        let parsed: IBig = from_str(&output).unwrap();
        assert_eq!(&parsed, int);
    }
}
