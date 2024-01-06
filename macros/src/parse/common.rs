use dashu_base::{ParseError, Sign};
use paste::paste;
use proc_macro2::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use quote::quote;

extern crate alloc;
use alloc::vec::Vec;

pub fn quote_bytes(bytes: &[u8]) -> Group {
    let bytes_stream: TokenStream = bytes
        .iter()
        .flat_map(|&b| {
            [
                TokenTree::Literal(Literal::u8_unsuffixed(b)),
                TokenTree::Punct(Punct::new(',', Spacing::Alone)),
            ]
        })
        .collect();
    Group::new(Delimiter::Bracket, bytes_stream)
}

macro_rules! define_array_converter {
    ($int:ty) => {
        paste! {
            /// Convert byte array to int array
            fn [<le_bytes_to_ $int _array>](bytes: &[u8]) -> Vec<$int> {
                const INT_SIZE: usize = <$int>::BITS as usize / 8;
                let mut ints = Vec::with_capacity(bytes.len() / INT_SIZE + 1);
                let mut chunks = bytes.chunks_exact(INT_SIZE);
                for chunk in &mut chunks {
                    ints.push(<$int>::from_le_bytes(chunk.try_into().unwrap()));
                }
                if !chunks.remainder().is_empty() {
                    // pad with zero
                    let rem = chunks.remainder();
                    let mut buffer = [0; INT_SIZE];
                    buffer[..rem.len()].copy_from_slice(rem);
                    ints.push(<$int>::from_le_bytes(buffer));
                }
                ints
            }

            /// Convert byte array to int array and generate tokens
            fn [<le_bytes_to_ $int _tokens>](bytes: &[u8], pad_to: usize) -> (Group, usize) {
                let mut ints = [<le_bytes_to_ $int _array>](bytes);
                let ints_len = ints.len();
                while ints.len() < pad_to {
                    ints.push(0);
                }
                let ints_stream: TokenStream = ints
                    .iter()
                    .flat_map(|&b| {
                        [
                            TokenTree::Literal(Literal::[<$int _unsuffixed>](b)),
                            TokenTree::Punct(Punct::new(',', Spacing::Alone)),
                        ]
                    })
                    .collect();
                (Group::new(Delimiter::Bracket, ints_stream), ints_len)
            }
        }
    };
}

define_array_converter!(u16);
define_array_converter!(u32);
define_array_converter!(u64);

/// This function generates a token tree whose content defines a data selector for
/// compatibility with different word size, and returns a reference to the proper data source.
pub fn quote_words(le_bytes: &[u8], embedded: bool) -> TokenStream {
    // Due to the limitations of Rust const generics, the arrays has to be padded to the same length.
    // See: https://users.rust-lang.org/t/how-to-use-associated-const-in-an-associated-type/104348
    let max_len = (le_bytes.len() + 1) / 2;
    let (u16_tokens, u16_len) = le_bytes_to_u16_tokens(le_bytes, max_len);
    let (u32_tokens, u32_len) = le_bytes_to_u32_tokens(le_bytes, max_len);
    let (u64_tokens, u64_len) = le_bytes_to_u64_tokens(le_bytes, max_len);

    let ns: TokenStream = if embedded {
        quote!(::dashu::integer)
    } else {
        quote!(::dashu_int)
    };

    quote! {{
        trait DataSource {
            type Int: 'static;
            const LEN: usize;
            const DATA: [Self::Int; #max_len];
        }
        struct DataSelector<const BITS: u32>;
        impl DataSource for DataSelector<16> {
            type Int = u16;
            const LEN: usize = #u16_len;
            const DATA: [u16; #max_len] = #u16_tokens;
        }
        impl DataSource for DataSelector<32> {
            type Int = u32;
            const LEN: usize = #u32_len;
            const DATA: [u32; #max_len] = #u32_tokens;
        }
        impl DataSource for DataSelector<64> {
            type Int = u64;
            const LEN: usize = #u64_len;
            const DATA: [u64; #max_len] = #u64_tokens;
        }

        type Select = DataSelector<{#ns::Word::BITS}>;
        // copy to make sure the pointer to the data is valid all the time.
        static DATA_COPY: [#ns::Word; Select::DATA.len()] = Select::DATA;

        // here slicing has to be implemented through the unsafe block, because range expression is not const.
        // See: https://users.rust-lang.org/t/constant-ranges-to-get-arrays-from-slices/67805
        unsafe { core::slice::from_raw_parts(DATA_COPY.as_ptr(), Select::LEN) }
    }}
}

pub fn quote_sign(embedded: bool, sign: Sign) -> TokenStream {
    if !embedded {
        match sign {
            Sign::Positive => quote! { ::dashu_base::Sign::Positive },
            Sign::Negative => quote! { ::dashu_base::Sign::Negative },
        }
    } else {
        match sign {
            Sign::Positive => quote! { ::dashu::base::Sign::Positive },
            Sign::Negative => quote! { ::dashu::base::Sign::Negative },
        }
    }
}

pub fn print_error_msg(error_type: ParseError) -> ! {
    match error_type {
        ParseError::NoDigits => panic!("Missing digits in the number components!"),
        ParseError::InvalidDigit => panic!("Invalid digits or syntax in the literal! Please refer to the documentation of this macro."),
        ParseError::UnsupportedRadix => panic!("The given radix is invalid or unsupported!"),
        ParseError::InconsistentRadix => panic!("Radix of different components are different!"),
    }
}

#[inline]
pub fn unwrap_with_error_msg<T>(result: Result<T, ParseError>) -> T {
    match result {
        Ok(v) => v,
        Err(e) => print_error_msg(e),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_bytes_convert() {
        let bytes: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let u16_arr = le_bytes_to_u16_array(&bytes);
        assert_eq!(u16_arr, &[0x201, 0x403, 0x605, 0x807, 9]);
        let u32_arr = le_bytes_to_u32_array(&bytes);
        assert_eq!(u32_arr, &[0x4030201, 0x8070605, 9]);
        let u64_arr = le_bytes_to_u64_array(&bytes);
        assert_eq!(u64_arr, &[0x807060504030201, 9]);
    }
}
