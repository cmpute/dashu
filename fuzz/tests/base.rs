//! Differential / fuzz tests for `dashu-base` primitive trait impls.
//!
//! These exercise the hand-written algorithms on primitive integer/float types
//! (`u8..u128`, `i8..i128`, `f32`/`f64`): `gcd`/`gcd_ext` (binary/modular hybrid +
//! Bézout identity), `sqrt_rem`/`cbrt_rem`, `sqrt`/`cbrt`, `div_rem`/`div_rem_euclid`,
//! `BitTest` (`bit`/`bit_len`, including the two's-complement extension for signed ints),
//! `PowerOfTwo::is_power_of_two`, `EstimatedLog2::log2_bounds` (std/no-std fixed-point
//! table), `FloatEncoding` (`encode`/`decode`, round-to-nearest-even), and
//! `next_up`/`next_down`.
//!
//! Most checks are self-consistency identities widened to `u128`/`i128` so reconstruction
//! cannot overflow. `gcd_ext`'s Bézout identity `g == a·x + b·y` is verified with
//! `rug::Integer` arithmetic, since `u128`-wide coefficients can overflow `i128`. Like the
//! other targets these are `#[ignore]`d proptests (manual, release-time; CI only `cargo
//! check`s this crate — see the `fuzz-check` workflow).
//!
//! `usize`/`isize` are intentionally omitted: their impls are pointer-width aliases of
//! `u32`/`u64` (or `i32`/`i64`), so the identical algorithm bodies are already covered.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test base -- --ignored --nocapture`

use dashu::base::{Approximation::*, EstimatedLog2, FloatEncoding};
use proptest::prelude::*;

/// Complete a `rug` reference expression into an owned `Integer` via `Assign`
/// (mirrors the helper in `fuzz/tests/integer.rs`).
fn rugc<S>(src: S) -> rug::Integer
where
    rug::Integer: rug::Assign<S>,
{
    let mut r = rug::Integer::new();
    rug::Assign::assign(&mut r, src);
    r
}

/// `log2_bounds` returns a strictly-enclosing `(lb, ub)` bracket around the true log2;
/// verify it in log space with a tolerance that absorbs f32 rounding (the bracket itself
/// is within a few f32 ulps, far below this).
#[allow(clippy::float_cmp)]
fn assert_log2_bracket(lb: f32, ub: f32, true_log2: f64) {
    assert!(lb <= ub);
    assert!(lb.is_finite() && ub.is_finite());
    let tol = 1e-5;
    assert!((lb as f64) - tol <= true_log2 && true_log2 <= (ub as f64) + tol);
}

// ===================== unsigned integer impls =====================
macro_rules! gen_uint {
    ($mod:ident, $T:ty) => {
        mod $mod {
            use super::*;
            use dashu::base::{
                BitTest, CubicRoot, CubicRootRem, DivRem, EstimatedLog2, ExtendedGcd, Gcd,
                PowerOfTwo, SquareRoot, SquareRootRem,
            };

            proptest! {
                #![proptest_config(fuzz::fuzz_config())]

                #[test]
                #[ignore]
                fn gcd(a in any::<$T>(), b in any::<$T>()) {
                    prop_assume!(a != 0 || b != 0); // gcd(0, 0) panics
                    let g = a.gcd(b);
                    // g is a common divisor…
                    prop_assert_eq!(a % g, 0);
                    prop_assert_eq!(b % g, 0);
                    // …and is commutative…
                    prop_assert_eq!(b.gcd(a), g);
                    // …and (with the Bézout identity below) that makes it the *greatest*:
                    // any common divisor divides a·x + b·y = g.
                    let (g2, x, y) = a.gcd_ext(b);
                    prop_assert_eq!(g2, g);
                    let bezout = {
                        let ra = rug::Integer::from(a);
                        let rb = rug::Integer::from(b);
                        let ax = rugc(&ra * &rug::Integer::from(x));
                        let by = rugc(&rb * &rug::Integer::from(y));
                        rugc(&ax + &by)
                    };
                    prop_assert_eq!(rug::Integer::from(g), bezout);
                }

                #[test]
                #[ignore]
                fn sqrt_rem(n in any::<$T>()) {
                    let (s, r) = n.sqrt_rem();
                    let s = s as u128;
                    let r = r as u128;
                    let n = n as u128;
                    prop_assert_eq!(s * s + r, n);
                    prop_assert!(r <= 2 * s); // ⟹ s² ≤ n < (s+1)², so s is the floor sqrt
                }

                #[test]
                #[ignore]
                fn cbrt_rem(n in any::<$T>()) {
                    let (c, r) = n.cbrt_rem();
                    let c = c as u128;
                    let next = c + 1;
                    prop_assert_eq!(c * c * c + (r as u128), n as u128);
                    prop_assert!((r as u128) < next * next * next - c * c * c);
                }

                #[test]
                #[ignore]
                fn sqrt_cbrt_match_rem(n in any::<$T>()) {
                    prop_assert_eq!(n.sqrt() as u128, n.sqrt_rem().0 as u128);
                    prop_assert_eq!(n.cbrt() as u128, n.cbrt_rem().0 as u128);
                }

                #[test]
                #[ignore]
                fn div_rem(a in any::<$T>(), b in any::<$T>()) {
                    prop_assume!(b != 0); // division by zero panics
                    let (q, r) = a.div_rem(b);
                    prop_assert_eq!((q as u128) * (b as u128) + (r as u128), a as u128);
                    prop_assert!((r as u128) < (b as u128));
                }

                #[test]
                #[ignore]
                fn bit_and_bit_len(x in any::<$T>(), n in 0usize..(<$T>::BITS as usize + 8)) {
                    let bits = <$T>::BITS as usize;
                    let expected = if n >= bits { false } else { (x >> n) & 1 != 0 };
                    prop_assert_eq!(x.bit(n), expected);
                    prop_assert_eq!(x.bit_len(), bits - x.leading_zeros() as usize);
                }

                #[test]
                #[ignore]
                fn is_power_of_two(x in any::<$T>()) {
                    // call via UFCS so we exercise dashu's `PowerOfTwo` impl, not the
                    // shadowing std inherent method of the same name
                    prop_assert_eq!(PowerOfTwo::is_power_of_two(&x), x.count_ones() == 1);
                }

                #[test]
                #[ignore]
                fn log2_bounds(x in any::<$T>()) {
                    let (lb, ub) = x.log2_bounds();
                    if x == 0 {
                        prop_assert!(lb.is_infinite() && lb.is_sign_negative() && lb == ub);
                    } else {
                        assert_log2_bracket(lb, ub, (x as f64).log2());
                    }
                }
            }
        }
    };
}

gen_uint!(u8_tests, u8);
gen_uint!(u16_tests, u16);
gen_uint!(u32_tests, u32);
gen_uint!(u64_tests, u64);
gen_uint!(u128_tests, u128);

// ===================== signed integer impls =====================
macro_rules! gen_int {
    ($mod:ident, $T:ty) => {
        mod $mod {
            use super::*;
            use dashu::base::{BitTest, DivRem, DivRemEuclid, EstimatedLog2};

            proptest! {
                #![proptest_config(fuzz::fuzz_config())]

                #[test]
                #[ignore]
                fn div_rem(a in any::<$T>(), b in any::<$T>()) {
                    prop_assume!(b != 0); // division by zero panics
                    prop_assume!(!(a == <$T>::MIN && b == -1)); // MIN / -1 overflows, like std
                    let (q, r) = a.div_rem(b);
                    prop_assert_eq!((q as i128) * (b as i128) + (r as i128), a as i128);
                    prop_assert!((r as i128).abs() < (b as i128).abs());
                }

                #[test]
                #[ignore]
                fn div_rem_euclid(a in any::<$T>(), b in any::<$T>()) {
                    prop_assume!(b != 0); // division by zero panics
                    prop_assume!(!(a == <$T>::MIN && b == -1)); // MIN / -1 overflows, like std
                    let (q, r) = a.div_rem_euclid(b);
                    // Euclidean `q·b` can underflow i128 when `a` is near MIN (e.g. exactly
                    // MIN-1), so reconstruct with wrapping arithmetic; for a correct result
                    // `q·b + r == a` holds exactly, hence also modulo 2^N.
                    prop_assert_eq!(q.wrapping_mul(b).wrapping_add(r), a);
                    prop_assert!((r as i128) >= 0 && (r as i128) < (b as i128).abs());
                    // matches std's per-method Euclidean division
                    prop_assert_eq!(q as i128, (a as i128).div_euclid(b as i128));
                    prop_assert_eq!(r as i128, (a as i128).rem_euclid(b as i128));
                }

                #[test]
                #[ignore]
                fn bit_and_bit_len(x in any::<$T>(), n in 0usize..(<$T>::BITS as usize + 8)) {
                    let bits = <$T>::BITS as usize;
                    // two's-complement bit: positions ≥ BITS are sign-extended (x < 0)
                    let expected = if n >= bits { x < 0 } else { ((x as i128) >> n) & 1 != 0 };
                    prop_assert_eq!(x.bit(n), expected);
                    let ua = x.unsigned_abs();
                    prop_assert_eq!(x.bit_len(), bits - ua.leading_zeros() as usize);
                }

                #[test]
                #[ignore]
                fn log2_bounds(x in any::<$T>()) {
                    let (lb, ub) = x.log2_bounds();
                    if x == 0 {
                        prop_assert!(lb.is_infinite() && lb.is_sign_negative() && lb == ub);
                    } else {
                        assert_log2_bracket(lb, ub, (x.unsigned_abs() as f64).log2());
                    }
                }
            }
        }
    };
}

gen_int!(i8_tests, i8);
gen_int!(i16_tests, i16);
gen_int!(i32_tests, i32);
gen_int!(i64_tests, i64);
gen_int!(i128_tests, i128);

// ===================== float impls =====================
proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    #[test]
    #[ignore]
    fn f32_log2_bounds(
        f in any::<u32>().prop_map(f32::from_bits).prop_filter("finite nonzero", |f| f.is_finite() && *f != 0.0)
    ) {
        let (lb, ub) = f.log2_bounds();
        assert_log2_bracket(lb, ub, (f.abs() as f64).log2());
    }

    #[test]
    #[ignore]
    fn f64_log2_bounds(
        f in any::<u64>().prop_map(f64::from_bits).prop_filter("finite nonzero", |f| f.is_finite() && *f != 0.0)
    ) {
        let (lb, ub) = f.log2_bounds();
        assert_log2_bracket(lb, ub, f.abs().log2());
    }

    #[test]
    #[ignore]
    fn f32_encode_decode_roundtrip(
        f in any::<u32>().prop_map(f32::from_bits).prop_filter("finite", |f| f.is_finite())
    ) {
        // every finite f32 is exactly representable, so decode-then-encode is Exact
        let (m, e) = f.decode().unwrap();
        prop_assert_eq!(f32::encode(m, e), Exact(f));
    }

    #[test]
    #[ignore]
    fn f64_encode_decode_roundtrip(
        f in any::<u64>().prop_map(f64::from_bits).prop_filter("finite", |f| f.is_finite())
    ) {
        let (m, e) = f.decode().unwrap();
        prop_assert_eq!(f64::encode(m, e), Exact(f));
    }

    #[test]
    #[ignore]
    fn f32_encode_never_nan(m in any::<i32>(), e in any::<i16>()) {
        // encode must never produce NaN (only finite values or signed infinity)
        prop_assert!(!f32::encode(m, e).value().is_nan());
    }

    #[test]
    #[ignore]
    fn f64_encode_never_nan(m in any::<i64>(), e in any::<i16>()) {
        prop_assert!(!f64::encode(m, e).value().is_nan());
    }

    #[test]
    #[ignore]
    fn f32_next_up_down_roundtrip(
        f in any::<u32>()
            .prop_map(f32::from_bits)
            .prop_filter("finite, not max (next_up(max) = +inf which next_down rejects)", |f| {
                f.is_finite() && *f != f32::MAX
            })
    ) {
        // next_up/next_down panic on NaN or infinity, so the filter excludes MAX (whose
        // next_up is +inf) and all non-finite bit patterns.
        prop_assert_eq!(dashu::base::utils::next_down(dashu::base::utils::next_up(f)), f);
        prop_assert!(dashu::base::utils::next_up(f) != f); // always moves
    }
}
