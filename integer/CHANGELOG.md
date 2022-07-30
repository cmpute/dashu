# Changelog

## 0.2.0 (WIP)

- Add a public API `as_words` to access internal representation of `UBig` and `IBig`.
- Add const constructors `from_word`, `from_dword` and a direct constructor `from_words` for `UBig` and `IBig`.
- Add `Mul` implementations between `Sign` and `UBig`/`IBig`
- Remove `ubig!` and `ibig!` macros from the crate, more powerful versions of them will be included in a separate `dashu-macro` crate.
- Change: function `zero()`, `one()`, `neg_one()` are changed to associated constants `ZERO`, `ONE`, `NEG_ONE`.
- Parsing: support underscore separater.
- Parsing: parsing a string with unsupported radix will now return an Err instead of `panic!`.
- Parsing: `from_str_with_radix_prefix` now also return the radix.

## 0.1.1

- Implemented modular inverse for the `Modulo` type.
- Implemented GCD and ExtendedGCD traits for `UBig` and `IBig`.

## 0.1.0

The code for big integer is ported from `ibig @ 0.3.5` with modifications stated in the [NOTICE](./NOTICE.md), the current MSRV for `dashu-int` is 1.57.
