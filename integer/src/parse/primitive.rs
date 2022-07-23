pub use crate::radix::digit_from_ascii_byte;
use crunchy::unroll;

/// Parse a primitive integer from a ASCII string in a const context given radix.
///
/// This function is not designed to be used in normal context, because the performance
/// is not optimal, so static life time is required for the input.
pub const fn parse_int_from_const_str<const R: u32>(bytes: &'static [u8]) -> Option<u128> {
    let mut result: u128 = 0;

    unroll! {
        for i in 0..128 { // at most 128 digits for a arbitrary-base number to be fit in u128
            if i < bytes.len() && bytes[i] != b'_' {
                let digit = match digit_from_ascii_byte(bytes[i], R) {
                    Some(d) => d,
                    None => return None,
                };
                if i < bytes.len() {
                    match result.checked_mul(R as u128) {
                        Some(prod) => match prod.checked_add(digit as u128) {
                            Some(sum) => { result = sum; },
                            None => return None,
                        },
                        None => return None,
                    }
                }
            }
        }
    }
    Some(result)
}

/// Parse a primitive integer from a ASCII string in a const context.
///
/// This function is not designed to be used in normal context, because the performance
/// is not optimal, so static life time is required for the input.
pub const fn parse_int_from_const_str_with_prefix(bytes: &'static [u8]) -> Option<u128> {
    if bytes.len() >= 2 {
        if bytes[0] == b'0' {
            // const workaround for str::strip_prefix
            let stripped = match bytes.split_first() {
                Some((_, s1)) => match s1.split_first() {
                    Some((_, s2)) => s2,
                    None => unreachable!(),
                },
                None => unreachable!(),
            };
            if bytes[1] == b'b' {
                return parse_int_from_const_str::<2>(stripped);
            } else if bytes[1] == b'o' {
                return parse_int_from_const_str::<8>(stripped);
            } else if bytes[1] == b'x' {
                return parse_int_from_const_str::<16>(stripped);
            }
        }
    }

    parse_int_from_const_str::<10>(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_int() {
        let test_cases = [
            ("0", Some(0)),
            ("1234", Some(1234)),
            ("12_34", Some(12_34)),
            ("0x1234", Some(0x1234)),
            ("0x_12_34", Some(0x_12_34)),
            ("0b1010", Some(0b1010)),
            ("0b_10_10", Some(0b_10_10)),
            // test overflow
            ("340282366920938463463374607431768211456", None),
            ("0x100000000000000000000000000000000", None),
            // test invalid
            ("_0x_12_34", None),
            ("abcd", None),
        ];

        for (src, int) in test_cases {
            assert_eq!(parse_int_from_const_str_with_prefix(src.as_bytes()), int);
        }
    }
}
