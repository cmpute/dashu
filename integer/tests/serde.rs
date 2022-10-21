mod helper_macros;
use serde_test::{assert_de_tokens, assert_tokens, Configure, Token};

#[test]
fn test_ubig_serde() {
    assert_tokens(&ubig!(0).compact(), &[Token::Seq { len: Some(0) }, Token::SeqEnd]);
    assert_tokens(&ubig!(0).readable(), &[Token::BorrowedStr("0")]);
    assert_de_tokens(&ubig!(0).compact(), &[Token::Seq { len: None }, Token::SeqEnd]);
    assert_de_tokens(&ubig!(0).readable(), &[Token::BorrowedStr("0")]);
    assert_tokens(
        &ubig!(17).compact(),
        &[Token::Seq { len: Some(1) }, Token::U64(17), Token::SeqEnd],
    );
    assert_tokens(&ubig!(17).readable(), &[Token::BorrowedStr("17")]);
    assert_de_tokens(
        &ubig!(17).compact(),
        &[Token::Seq { len: None }, Token::U8(17), Token::SeqEnd],
    );
    assert_de_tokens(&ubig!(17).readable(), &[Token::BorrowedStr("17")]);
    assert_tokens(
        &ubig!(0x123451234567890abcdef).compact(),
        &[
            Token::Seq { len: Some(2) },
            Token::U64(0x1234567890abcdef),
            Token::U64(0x12345),
            Token::SeqEnd,
        ],
    );
    assert_tokens(
        &ubig!(0x123451234567890abcdef).readable(),
        &[Token::BorrowedStr("1375482783624620011146735")],
    );
    assert_de_tokens(
        &ubig!(0x123451234567890abcdef).compact(),
        &[
            Token::Seq { len: None },
            Token::U64(0x1234567890abcdef),
            Token::U64(0x12345),
            Token::SeqEnd,
        ],
    );
    assert_de_tokens(
        &ubig!(0x123451234567890abcdef).readable(),
        &[Token::BorrowedStr("0x123451234567890abcdef")],
    );
}

#[test]
fn test_ibig_serde() {
    assert_tokens(
        &ibig!(0).compact(),
        &[
            Token::Tuple { len: 2 },
            Token::Bool(false),
            Token::Seq { len: Some(0) },
            Token::SeqEnd,
            Token::TupleEnd,
        ],
    );
    assert_tokens(&ibig!(0).readable(), &[Token::BorrowedStr("0")]);
    assert_de_tokens(
        &ibig!(0).compact(),
        &[
            Token::Seq { len: None },
            Token::Bool(true),
            Token::Seq { len: None },
            Token::SeqEnd,
            Token::SeqEnd,
        ],
    );
    assert_de_tokens(&ibig!(0).readable(), &[Token::BorrowedStr("0")]);
    assert_tokens(
        &ibig!(17).compact(),
        &[
            Token::Tuple { len: 2 },
            Token::Bool(false),
            Token::Seq { len: Some(1) },
            Token::U64(17),
            Token::SeqEnd,
            Token::TupleEnd,
        ],
    );
    assert_tokens(&ibig!(17).readable(), &[Token::BorrowedStr("17")]);
    assert_de_tokens(&ibig!(17).readable(), &[Token::BorrowedStr("0x11")]);
    assert_tokens(
        &ibig!(-17).compact(),
        &[
            Token::Tuple { len: 2 },
            Token::Bool(true),
            Token::Seq { len: Some(1) },
            Token::U64(17),
            Token::SeqEnd,
            Token::TupleEnd,
        ],
    );
    assert_tokens(&ibig!(-17).readable(), &[Token::BorrowedStr("-17")]);
    assert_de_tokens(&ibig!(-17).readable(), &[Token::BorrowedStr("-0x11")]);
}
