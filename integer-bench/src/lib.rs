//! Shared helpers and the generic backend abstraction for the dashu-int
//! comparison benchmarks (`primitive`, `small_int`, `workload`, `shrinker`).
//!
//! Each criterion bench body is written once over [`Backend`] and run for every
//! backend, with the backend name as a `BenchmarkId` dimension, so one run
//! reports them side-by-side. The pure-Rust backends (dashu, ibig, num,
//! malachite) are always available; rug (GNU GMP) is added under the `gmp`
//! feature, since it needs the GMP toolchain.

#![allow(dead_code)]

use dashu_int::{IBig, UBig};
use rand_v08::prelude::*;
use rand_v08::rngs::StdRng;

/// Coarse value-magnitude classes used to drive the bench parameter sweeps.
///
/// Intent is to exercise the small / inline-magnitude ranges (zero, one word,
/// two words, just-over-inline) rather than only the large-buffer paths that
/// the bit-width sweep covers.
#[derive(Clone, Copy, Debug)]
pub enum ValueClass {
    /// Exactly zero. Common in real workloads (initial accumulators, defaults).
    Zero,
    /// Fits in a single `Word` (≤ 64 bits on 64-bit targets). Single-word inline.
    OneWord,
    /// Needs both inline words (65–128 bits). Still inline, but the upper word
    /// is meaningful.
    TwoWord,
    /// Just past the inline boundary (129–256 bits). Heap-allocated but tiny.
    JustOverInline,
    /// Medium (~1024 bits). Multi-limb but still in fast-path territory for
    /// schoolbook arithmetic.
    Mid,
    /// Large (~10k bits). Past the ~1-kbit crossover where the GMP-backed and
    /// asymptotically-faster libraries pull ahead.
    Large,
}

impl ValueClass {
    pub const ALL: &'static [ValueClass] = &[
        ValueClass::Zero,
        ValueClass::OneWord,
        ValueClass::TwoWord,
        ValueClass::JustOverInline,
        ValueClass::Mid,
        ValueClass::Large,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ValueClass::Zero => "zero",
            ValueClass::OneWord => "one_word",
            ValueClass::TwoWord => "two_word",
            ValueClass::JustOverInline => "just_over_inline",
            ValueClass::Mid => "mid",
            ValueClass::Large => "large",
        }
    }
}

/// Sample a `UBig` from the given class.
pub fn sample_ubig<R: Rng>(class: ValueClass, rng: &mut R) -> UBig {
    match class {
        ValueClass::Zero => UBig::from(0u32),
        // Non-zero so the inline path is exercised meaningfully.
        ValueClass::OneWord => UBig::from(rng.gen::<u64>() | 1),
        ValueClass::TwoWord => {
            let lo: u64 = rng.gen();
            let hi: u64 = rng.gen::<u64>() | (1 << 63); // force the top word non-empty
            (UBig::from(hi) << 64) + UBig::from(lo)
        }
        ValueClass::JustOverInline => random_ubig(192, rng),
        ValueClass::Mid => random_ubig(1024, rng),
        ValueClass::Large => random_ubig(10_000, rng),
    }
}

/// Same shape as `sample_ubig`, but produces `IBig` and alternates sign for
/// even/odd RNG draws so negative paths get exercised.
pub fn sample_ibig<R: Rng>(class: ValueClass, rng: &mut R) -> IBig {
    let mag = IBig::from(sample_ubig(class, rng));
    if rng.gen::<bool>() {
        -mag
    } else {
        mag
    }
}

/// Helper modelled on the one in `primitive.rs`: a uniformly distributed UBig
/// of approximately `bits` bits (at least 2^(bits-1)).
pub fn random_ubig<R: Rng>(bits: usize, rng: &mut R) -> UBig {
    rng.gen_range(UBig::ONE << (bits - 1)..UBig::ONE << bits)
}

/// Draw a class from a distribution that approximates a shrinker-style
/// workload: small values dominate, large values are rare. Numbers sum to 100.
pub fn mixed_class<R: Rng>(rng: &mut R) -> ValueClass {
    let r: u32 = rng.gen_range(0..100);
    match r {
        0..=4 => ValueClass::Zero,
        5..=64 => ValueClass::OneWord,
        65..=89 => ValueClass::TwoWord,
        90..=96 => ValueClass::JustOverInline,
        97..=98 => ValueClass::Mid,
        _ => ValueClass::Large,
    }
}

pub fn seeded_rng() -> StdRng {
    StdRng::seed_from_u64(0x0DA5_4BE4)
}

// Cross-library sampling helpers. Every backend samples by drawing a dashu
// value (so the RNG is consumed identically and magnitudes line up across
// libraries) and then converting it. The conversion goes through a hex string,
// which every candidate library parses in O(n) — fine for setup-only work.

/// Hex digits of an unsigned dashu value.
#[allow(dead_code)]
fn ubig_hex(u: &UBig) -> String {
    u.in_radix(16).to_string()
}

/// `(is_negative, hex-of-magnitude)` for a signed dashu value.
#[allow(dead_code)]
fn ibig_sign_hex(i: IBig) -> (bool, String) {
    let neg = i < IBig::from(0i32);
    (neg, i.unsigned_abs().in_radix(16).to_string())
}

// ---------------------------------------------------------------------------
// Rug helper — only compiled under the `gmp` feature, used by the `Rug` backend
// below. The rug samplers (in the `Rug` impl) draw a dashu value and convert it
// here, exactly like the other backends, so magnitudes track `sample_ubig` /
// `sample_ibig` automatically.
// ---------------------------------------------------------------------------

#[cfg(feature = "gmp")]
#[allow(unused_imports)]
pub use rug_side::*;

#[cfg(feature = "gmp")]
mod rug_side {
    use dashu_int::UBig;
    use rug::Integer as RugInt;

    /// Convert a `UBig` of any size to a `rug::Integer`. Goes via the byte
    /// representation rather than the limb words because rug exposes
    /// `Integer::from_digits` for that, and the conversion is one-off (used
    /// only in bench setup, never in the timed loop).
    pub fn ubig_to_rug(u: &UBig) -> RugInt {
        let bytes = u.to_be_bytes();
        RugInt::from_digits(&bytes, rug::integer::Order::Msf)
    }
}

// ===========================================================================
// Generic backend abstraction.
//
// Each criterion bench body is written once over `Backend` and run against
// every enabled backend, with the backend name as a `BenchmarkId` dimension,
// so one `cargo bench` run reports them side-by-side. The pure-Rust backends
// (dashu, ibig, num-bigint, malachite) are always built; rug (GNU GMP) is
// added under the `gmp` feature.
//
// This revives the trait-based, multi-library approach of the top-level
// `benchmark/` harness while emitting criterion measurements.
//
// `Backend` selects the concrete unsigned/signed integer types and samplers;
// `BenchInt` (+ `UnsignedInt` / `SignedInt`) supply the operations the bench
// bodies call. The unsigned/signed split mirrors dashu's real `UBig` / `IBig`
// divide; rug uses its single signed `Integer` for both associated types,
// while num / malachite / ibig have a `BigUint`/`BigInt`-style split like dashu.
//
// Some libraries' by-reference operators return lazy "incomplete-computation"
// values rather than an owned integer, so the operations can't be expressed
// through the std `Add`/`Sub`/... bounds directly — each is a method here,
// exactly as the prior-art `Natural` trait did with `mul_ref`. Every backend
// samples by drawing a dashu value and converting it, so magnitudes line up
// point-for-point across libraries.
// ===========================================================================

use core::fmt::Display;
use core::hash::Hash;
use dashu_int::fast_div::ConstDivisor;
use dashu_int::ops::{ExtendedGcd, Gcd, UnsignedAbs};

/// Operations shared by the unsigned and signed bench integer types.
///
/// Every method returns an owned value or mutates in place, so it covers both
/// dashu (operators already return owned) and rug (operators return a lazy
/// incomplete value that the impl finalises with `Integer::from`).
pub trait BenchInt: Clone + Ord + Hash + Display {
    fn parse(s: &str) -> Self;

    fn add_ref(&self, rhs: &Self) -> Self;
    fn sub_ref(&self, rhs: &Self) -> Self;
    fn mul_ref(&self, rhs: &Self) -> Self;
    fn div_ref(&self, rhs: &Self) -> Self;
    fn bitand_ref(&self, rhs: &Self) -> Self;
    fn bitxor_ref(&self, rhs: &Self) -> Self;
    fn shl_ref(&self, bits: usize) -> Self;
    fn shr_ref(&self, bits: usize) -> Self;

    // The `*_assign_ref` ops default to `*self = self.op_ref(rhs)`. Backends
    // with a native in-place operator (dashu, rug, ...) override them so the
    // benches measure the real `+=` path; libraries without one fall back to
    // the allocating form, which is what they'd do in practice anyway.
    fn add_assign_ref(&mut self, rhs: &Self) {
        *self = self.add_ref(rhs);
    }
    fn sub_assign_ref(&mut self, rhs: &Self) {
        *self = self.sub_ref(rhs);
    }
    fn bitxor_assign_ref(&mut self, rhs: &Self) {
        *self = self.bitxor_ref(rhs);
    }
}

/// Construction and primitive `+=` for the unsigned type. `UBig` never builds
/// from a signed primitive, so only the unsigned constructors live here.
pub trait UnsignedInt: BenchInt {
    fn from_u64(v: u64) -> Self;
    fn from_u128(v: u128) -> Self;
    // Default to constructing the RHS and adding; backends with a native
    // `+= u64` / `+= u128` override.
    fn add_assign_u64(&mut self, rhs: u64) {
        *self = self.add_ref(&Self::from_u64(rhs));
    }
    fn add_assign_u128(&mut self, rhs: u128) {
        *self = self.add_ref(&Self::from_u128(rhs));
    }
}

/// Construction, primitive `+=`, and `TryInto<i128>` for the signed type.
/// Includes unsigned constructors because the signed benches build `IBig`
/// values from `u64` / `u128` magnitudes (e.g. `1u128 << exp`).
pub trait SignedInt: BenchInt {
    fn from_i64(v: i64) -> Self;
    fn from_i128(v: i128) -> Self;
    fn from_u64(v: u64) -> Self;
    fn from_u128(v: u128) -> Self;
    fn try_to_i128(&self) -> Option<i128>;
    fn add_assign_i64(&mut self, rhs: i64) {
        *self = self.add_ref(&Self::from_i64(rhs));
    }
    fn add_assign_i128(&mut self, rhs: i128) {
        *self = self.add_ref(&Self::from_i128(rhs));
    }
}

/// A bignum implementation under test. `Dashu`, `Ibig`, `Num` and `Malachite`
/// are always available; `Rug` is added under the `gmp` feature.
pub trait Backend {
    /// Tag used as the `BenchmarkId` group/function name so dashu and rug
    /// measurements sit side-by-side in one criterion report.
    const NAME: &'static str;
    type Unsigned: UnsignedInt;
    type Signed: SignedInt;

    fn sample_unsigned<R: Rng>(class: ValueClass, rng: &mut R) -> Self::Unsigned;
    fn sample_signed<R: Rng>(class: ValueClass, rng: &mut R) -> Self::Signed;

    /// `|value|` as the unsigned type — dashu's `IBig::unsigned_abs() -> UBig`,
    /// rug's `Integer::abs()`. Consumes `value` so no extra clone enters the
    /// timed path (the sort-key pattern is `(a - b).magnitude()`).
    fn magnitude(value: Self::Signed) -> Self::Unsigned;

    /// Reinterpret an unsigned magnitude as the signed type. dashu:
    /// `IBig::from(UBig)`; rug: identity.
    fn unsigned_to_signed(value: Self::Unsigned) -> Self::Signed;
}

// ---- dashu impls ----------------------------------------------------------

impl BenchInt for UBig {
    fn parse(s: &str) -> Self {
        s.parse().unwrap()
    }
    fn add_ref(&self, rhs: &Self) -> Self {
        self + rhs
    }
    fn sub_ref(&self, rhs: &Self) -> Self {
        self - rhs
    }
    fn mul_ref(&self, rhs: &Self) -> Self {
        self * rhs
    }
    fn div_ref(&self, rhs: &Self) -> Self {
        self / rhs
    }
    fn bitand_ref(&self, rhs: &Self) -> Self {
        self & rhs
    }
    fn bitxor_ref(&self, rhs: &Self) -> Self {
        self ^ rhs
    }
    fn shl_ref(&self, bits: usize) -> Self {
        self << bits
    }
    fn shr_ref(&self, bits: usize) -> Self {
        self >> bits
    }
    fn add_assign_ref(&mut self, rhs: &Self) {
        *self += rhs;
    }
    fn sub_assign_ref(&mut self, rhs: &Self) {
        *self -= rhs;
    }
    fn bitxor_assign_ref(&mut self, rhs: &Self) {
        *self ^= rhs;
    }
}

impl UnsignedInt for UBig {
    fn from_u64(v: u64) -> Self {
        UBig::from(v)
    }
    fn from_u128(v: u128) -> Self {
        UBig::from(v)
    }
    fn add_assign_u64(&mut self, rhs: u64) {
        *self += rhs;
    }
    fn add_assign_u128(&mut self, rhs: u128) {
        *self += rhs;
    }
}

impl BenchInt for IBig {
    fn parse(s: &str) -> Self {
        s.parse().unwrap()
    }
    fn add_ref(&self, rhs: &Self) -> Self {
        self + rhs
    }
    fn sub_ref(&self, rhs: &Self) -> Self {
        self - rhs
    }
    fn mul_ref(&self, rhs: &Self) -> Self {
        self * rhs
    }
    fn div_ref(&self, rhs: &Self) -> Self {
        self / rhs
    }
    fn bitand_ref(&self, rhs: &Self) -> Self {
        self & rhs
    }
    fn bitxor_ref(&self, rhs: &Self) -> Self {
        self ^ rhs
    }
    fn shl_ref(&self, bits: usize) -> Self {
        self << bits
    }
    fn shr_ref(&self, bits: usize) -> Self {
        self >> bits
    }
    fn add_assign_ref(&mut self, rhs: &Self) {
        *self += rhs;
    }
    fn sub_assign_ref(&mut self, rhs: &Self) {
        *self -= rhs;
    }
    fn bitxor_assign_ref(&mut self, rhs: &Self) {
        *self ^= rhs;
    }
}

impl SignedInt for IBig {
    fn from_i64(v: i64) -> Self {
        IBig::from(v)
    }
    fn from_i128(v: i128) -> Self {
        IBig::from(v)
    }
    fn from_u64(v: u64) -> Self {
        IBig::from(v)
    }
    fn from_u128(v: u128) -> Self {
        IBig::from(v)
    }
    fn try_to_i128(&self) -> Option<i128> {
        i128::try_from(self).ok()
    }
    fn add_assign_i64(&mut self, rhs: i64) {
        *self += rhs;
    }
    fn add_assign_i128(&mut self, rhs: i128) {
        *self += rhs;
    }
}

/// dashu backend: distinct `UBig` / `IBig` types.
pub struct Dashu;

impl Backend for Dashu {
    const NAME: &'static str = "dashu";
    type Unsigned = UBig;
    type Signed = IBig;

    fn sample_unsigned<R: Rng>(class: ValueClass, rng: &mut R) -> UBig {
        sample_ubig(class, rng)
    }
    fn sample_signed<R: Rng>(class: ValueClass, rng: &mut R) -> IBig {
        sample_ibig(class, rng)
    }
    fn magnitude(value: IBig) -> UBig {
        value.unsigned_abs()
    }
    fn unsigned_to_signed(value: UBig) -> IBig {
        IBig::from(value)
    }
}

// ---------------------------------------------------------------------------
// Extra surface used only by the `primitive` bit-width-sweep bench: gcd, pow,
// radix conversions, and modular arithmetic. Kept in dedicated traits so the
// core `BenchInt` / `Backend` used by the other suites stays small.
//
// The modular ops (`mod_mul` / `mod_pow`) are written so every backend does
// the same thing it would naturally do, with nothing amortised away: a plain
// multiply-then-reduce, and the library's native one-shot modpow.
// ---------------------------------------------------------------------------

/// Operations the `primitive` bench needs beyond the common `BenchInt` set.
/// All on the unsigned type (the sweep is `UBig`-only).
///
/// `pow_exp` / `gcd` / `gcd_ext_blackbox` have portable default
/// implementations (square-and-multiply, Euclid) so any library can take part
/// in the bench; backends with a faster native routine (GMP's gcd, etc.)
/// override them so that bench reflects the real thing. `to_radix_string` /
/// `from_radix` / `write_hex` are required — every candidate library has
/// radix conversion.
pub trait PrimitiveInt: UnsignedInt {
    fn to_radix_string(&self, radix: u32) -> String;
    fn from_radix(s: &str, radix: u32) -> Self;
    fn write_hex(&self, out: &mut String);

    fn pow_exp(&self, exp: usize) -> Self {
        // Square-and-multiply.
        let mut result = Self::from_u64(1);
        let mut base = self.clone();
        let mut e = exp;
        while e > 0 {
            if e & 1 == 1 {
                result = result.mul_ref(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul_ref(&base);
            }
        }
        result
    }

    fn gcd(&self, rhs: &Self) -> Self {
        // Euclid via div/mul/sub (no `rem` in `BenchInt`); on the unsigned type.
        let zero = Self::from_u64(0);
        let mut a = self.clone();
        let mut b = rhs.clone();
        while b != zero {
            let q = a.div_ref(&b);
            let r = a.sub_ref(&q.mul_ref(&b));
            a = b;
            b = r;
        }
        a
    }

    /// Compute the extended gcd and discard the result through `black_box`.
    /// The cofactor types differ between backends, so only timing is compared.
    /// Defaults to the plain `gcd` (libraries with a native extended gcd
    /// override to measure the cofactor work too).
    fn gcd_ext_blackbox(&self, rhs: &Self) {
        core::hint::black_box(self.gcd(rhs));
    }
}

/// Backend extension for the `primitive` bench: a bit-width sampler and modular
/// arithmetic. Separate from [`Backend`] so the other suites don't carry it.
///
/// The modular ops take the modulus directly and are written the way each
/// library actually exposes them, with no setup amortised away: `mod_mul` is
/// the plain "multiply then reduce", and `mod_pow` calls the library's native
/// modular exponentiation (building any per-modulus context inside the call,
/// since that is part of a one-shot modpow's real cost).
pub trait PrimitiveBackend: Backend {
    /// A non-negative value of approximately `bits` bits, magnitude-identical
    /// across backends for a given RNG state.
    fn sample_unsigned_bits<R: Rng>(bits: usize, rng: &mut R) -> Self::Unsigned;

    /// `(a * b) mod m`, multiply-then-reduce — the same shape for every backend.
    fn mod_mul(a: &Self::Unsigned, b: &Self::Unsigned, m: &Self::Unsigned) -> Self::Unsigned;

    /// `a^exp mod m` via the library's native modular exponentiation.
    fn mod_pow(a: &Self::Unsigned, exp: &Self::Unsigned, m: &Self::Unsigned) -> Self::Unsigned;
}

impl PrimitiveInt for UBig {
    fn pow_exp(&self, exp: usize) -> Self {
        self.pow(exp)
    }
    fn gcd(&self, rhs: &Self) -> Self {
        Gcd::gcd(self, rhs)
    }
    fn gcd_ext_blackbox(&self, rhs: &Self) {
        core::hint::black_box(ExtendedGcd::gcd_ext(self, rhs));
    }
    fn to_radix_string(&self, radix: u32) -> String {
        self.in_radix(radix).to_string()
    }
    fn from_radix(s: &str, radix: u32) -> Self {
        UBig::from_str_radix(s, radix).unwrap()
    }
    fn write_hex(&self, out: &mut String) {
        use core::fmt::Write;
        write!(out, "{:x}", self).unwrap();
    }
}

impl PrimitiveBackend for Dashu {
    fn sample_unsigned_bits<R: Rng>(bits: usize, rng: &mut R) -> UBig {
        random_ubig(bits, rng)
    }
    fn mod_mul(a: &UBig, b: &UBig, m: &UBig) -> UBig {
        a * b % m
    }
    fn mod_pow(a: &UBig, exp: &UBig, m: &UBig) -> UBig {
        // dashu's modpow goes through a ConstDivisor; build it here so the
        // one-shot cost (divisor setup included) is what gets measured, like
        // the others' native modpow.
        ConstDivisor::new(m.clone())
            .reduce(a.clone())
            .pow(exp)
            .residue()
    }
}

// ---- rug impls ------------------------------------------------------------

#[cfg(feature = "gmp")]
pub use rug_backend::Rug;

#[cfg(feature = "gmp")]
mod rug_backend {
    use super::{
        random_ubig, sample_ibig, sample_ubig, ubig_to_rug, Backend, BenchInt, PrimitiveBackend,
        PrimitiveInt, SignedInt, UnsignedInt, ValueClass,
    };
    use dashu_int::ops::UnsignedAbs;
    use dashu_int::IBig;
    use rand_v08::Rng;
    use rug::Integer;

    impl BenchInt for Integer {
        fn parse(s: &str) -> Self {
            s.parse().unwrap()
        }
        fn add_ref(&self, rhs: &Self) -> Self {
            Integer::from(self + rhs)
        }
        fn sub_ref(&self, rhs: &Self) -> Self {
            Integer::from(self - rhs)
        }
        fn mul_ref(&self, rhs: &Self) -> Self {
            Integer::from(self * rhs)
        }
        fn div_ref(&self, rhs: &Self) -> Self {
            Integer::from(self / rhs)
        }
        fn bitand_ref(&self, rhs: &Self) -> Self {
            Integer::from(self & rhs)
        }
        fn bitxor_ref(&self, rhs: &Self) -> Self {
            Integer::from(self ^ rhs)
        }
        fn shl_ref(&self, bits: usize) -> Self {
            Integer::from(self << bits as u32)
        }
        fn shr_ref(&self, bits: usize) -> Self {
            Integer::from(self >> bits as u32)
        }
        fn add_assign_ref(&mut self, rhs: &Self) {
            *self += rhs;
        }
        fn sub_assign_ref(&mut self, rhs: &Self) {
            *self -= rhs;
        }
        fn bitxor_assign_ref(&mut self, rhs: &Self) {
            *self ^= rhs;
        }
    }

    impl UnsignedInt for Integer {
        fn from_u64(v: u64) -> Self {
            Integer::from(v)
        }
        fn from_u128(v: u128) -> Self {
            Integer::from(v)
        }
        fn add_assign_u64(&mut self, rhs: u64) {
            *self += rhs;
        }
        fn add_assign_u128(&mut self, rhs: u128) {
            *self += rhs;
        }
    }

    impl SignedInt for Integer {
        fn from_i64(v: i64) -> Self {
            Integer::from(v)
        }
        fn from_i128(v: i128) -> Self {
            Integer::from(v)
        }
        fn from_u64(v: u64) -> Self {
            Integer::from(v)
        }
        fn from_u128(v: u128) -> Self {
            Integer::from(v)
        }
        fn try_to_i128(&self) -> Option<i128> {
            i128::try_from(self).ok()
        }
        fn add_assign_i64(&mut self, rhs: i64) {
            *self += rhs;
        }
        fn add_assign_i128(&mut self, rhs: i128) {
            *self += rhs;
        }
    }

    impl PrimitiveInt for Integer {
        fn pow_exp(&self, exp: usize) -> Self {
            Integer::from(rug::ops::Pow::pow(self, exp as u32))
        }
        fn gcd(&self, rhs: &Self) -> Self {
            Integer::from(self.gcd_ref(rhs))
        }
        fn gcd_ext_blackbox(&self, rhs: &Self) {
            core::hint::black_box(<(Integer, Integer)>::from(self.extended_gcd_ref(rhs)));
        }
        fn to_radix_string(&self, radix: u32) -> String {
            self.to_string_radix(radix as i32)
        }
        fn from_radix(s: &str, radix: u32) -> Self {
            Integer::from(Integer::parse_radix(s, radix as i32).unwrap())
        }
        fn write_hex(&self, out: &mut String) {
            use core::fmt::Write;
            write!(out, "{:x}", self).unwrap();
        }
    }

    /// rug backend: a single `Integer` serves as both unsigned and signed.
    pub struct Rug;

    impl Backend for Rug {
        const NAME: &'static str = "rug";
        type Unsigned = Integer;
        type Signed = Integer;

        fn sample_unsigned<R: Rng>(class: ValueClass, rng: &mut R) -> Integer {
            ubig_to_rug(&sample_ubig(class, rng))
        }
        fn sample_signed<R: Rng>(class: ValueClass, rng: &mut R) -> Integer {
            let i = sample_ibig(class, rng);
            let neg = i < IBig::from(0i32);
            let mag = ubig_to_rug(&i.unsigned_abs());
            if neg {
                -mag
            } else {
                mag
            }
        }
        fn magnitude(value: Integer) -> Integer {
            value.abs()
        }
        fn unsigned_to_signed(value: Integer) -> Integer {
            value
        }
    }

    impl PrimitiveBackend for Rug {
        fn sample_unsigned_bits<R: Rng>(bits: usize, rng: &mut R) -> Integer {
            // Go through the dashu sampler so magnitudes (and RNG draws) match
            // the dashu side point-for-point.
            ubig_to_rug(&random_ubig(bits, rng))
        }
        fn mod_mul(a: &Integer, b: &Integer, m: &Integer) -> Integer {
            Integer::from(a * b) % m
        }
        fn mod_pow(a: &Integer, exp: &Integer, m: &Integer) -> Integer {
            Integer::from(a.pow_mod_ref(exp, m).unwrap())
        }
    }
}

// ---- ibig impls -----------------------------------------------------------

pub use ibig_backend::Ibig;

mod ibig_backend {
    use super::{
        ibig_sign_hex, sample_ibig, sample_ubig, ubig_hex, Backend, BenchInt, PrimitiveBackend,
        PrimitiveInt, SignedInt, UnsignedInt, ValueClass,
    };
    use core::fmt::Write as _;
    use ibig::modular::ModuloRing;
    use ibig::ops::UnsignedAbs;
    use ibig::{IBig as I, UBig as U};
    use rand_v08::Rng;

    fn to_u(u: &super::UBig) -> U {
        U::from_str_radix(&ubig_hex(u), 16).unwrap()
    }
    fn to_s(i: super::IBig) -> I {
        let (neg, hex) = ibig_sign_hex(i);
        let mag = I::from(U::from_str_radix(&hex, 16).unwrap());
        if neg {
            -mag
        } else {
            mag
        }
    }

    impl BenchInt for U {
        fn parse(s: &str) -> Self {
            s.parse().unwrap()
        }
        fn add_ref(&self, rhs: &Self) -> Self {
            self + rhs
        }
        fn sub_ref(&self, rhs: &Self) -> Self {
            self - rhs
        }
        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
        fn div_ref(&self, rhs: &Self) -> Self {
            self / rhs
        }
        fn bitand_ref(&self, rhs: &Self) -> Self {
            self & rhs
        }
        fn bitxor_ref(&self, rhs: &Self) -> Self {
            self ^ rhs
        }
        fn shl_ref(&self, bits: usize) -> Self {
            self << bits
        }
        fn shr_ref(&self, bits: usize) -> Self {
            self >> bits
        }
        fn add_assign_ref(&mut self, rhs: &Self) {
            *self += rhs;
        }
        fn sub_assign_ref(&mut self, rhs: &Self) {
            *self -= rhs;
        }
        fn bitxor_assign_ref(&mut self, rhs: &Self) {
            *self ^= rhs;
        }
    }

    impl UnsignedInt for U {
        fn from_u64(v: u64) -> Self {
            U::from(v)
        }
        fn from_u128(v: u128) -> Self {
            U::from(v)
        }
        fn add_assign_u64(&mut self, rhs: u64) {
            *self += rhs;
        }
        fn add_assign_u128(&mut self, rhs: u128) {
            *self += rhs;
        }
    }

    impl BenchInt for I {
        fn parse(s: &str) -> Self {
            s.parse().unwrap()
        }
        fn add_ref(&self, rhs: &Self) -> Self {
            self + rhs
        }
        fn sub_ref(&self, rhs: &Self) -> Self {
            self - rhs
        }
        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
        fn div_ref(&self, rhs: &Self) -> Self {
            self / rhs
        }
        fn bitand_ref(&self, rhs: &Self) -> Self {
            self & rhs
        }
        fn bitxor_ref(&self, rhs: &Self) -> Self {
            self ^ rhs
        }
        fn shl_ref(&self, bits: usize) -> Self {
            self << bits
        }
        fn shr_ref(&self, bits: usize) -> Self {
            self >> bits
        }
        fn add_assign_ref(&mut self, rhs: &Self) {
            *self += rhs;
        }
        fn sub_assign_ref(&mut self, rhs: &Self) {
            *self -= rhs;
        }
        fn bitxor_assign_ref(&mut self, rhs: &Self) {
            *self ^= rhs;
        }
    }

    impl SignedInt for I {
        fn from_i64(v: i64) -> Self {
            I::from(v)
        }
        fn from_i128(v: i128) -> Self {
            I::from(v)
        }
        fn from_u64(v: u64) -> Self {
            I::from(v)
        }
        fn from_u128(v: u128) -> Self {
            I::from(v)
        }
        fn try_to_i128(&self) -> Option<i128> {
            i128::try_from(self).ok()
        }
        fn add_assign_i64(&mut self, rhs: i64) {
            *self += rhs;
        }
        fn add_assign_i128(&mut self, rhs: i128) {
            *self += rhs;
        }
    }

    impl PrimitiveInt for U {
        fn to_radix_string(&self, radix: u32) -> String {
            self.in_radix(radix).to_string()
        }
        fn from_radix(s: &str, radix: u32) -> Self {
            U::from_str_radix(s, radix).unwrap()
        }
        fn write_hex(&self, out: &mut String) {
            write!(out, "{:x}", self).unwrap();
        }
        fn pow_exp(&self, exp: usize) -> Self {
            // inherent `UBig::pow` (shadows the trait method of the same area)
            U::pow(self, exp)
        }
        fn gcd(&self, rhs: &Self) -> Self {
            U::gcd(self, rhs)
        }
        fn gcd_ext_blackbox(&self, rhs: &Self) {
            core::hint::black_box(self.extended_gcd(rhs));
        }
    }

    /// ibig backend: pure-Rust `UBig` / `IBig`, dashu's ancestor.
    pub struct Ibig;

    impl Backend for Ibig {
        const NAME: &'static str = "ibig";
        type Unsigned = U;
        type Signed = I;

        fn sample_unsigned<R: Rng>(class: ValueClass, rng: &mut R) -> U {
            to_u(&sample_ubig(class, rng))
        }
        fn sample_signed<R: Rng>(class: ValueClass, rng: &mut R) -> I {
            to_s(sample_ibig(class, rng))
        }
        fn magnitude(value: I) -> U {
            value.unsigned_abs()
        }
        fn unsigned_to_signed(value: U) -> I {
            I::from(value)
        }
    }

    // Plain `% m` modular arithmetic (ibig has a `modular` module, but the
    // benches use the uniform reduce-each-step shape so every non-dashu backend
    // is measured the same way).
    impl PrimitiveBackend for Ibig {
        fn sample_unsigned_bits<R: Rng>(bits: usize, rng: &mut R) -> U {
            to_u(&super::random_ubig(bits, rng))
        }
        fn mod_mul(a: &U, b: &U, m: &U) -> U {
            a * b % m
        }
        fn mod_pow(a: &U, exp: &U, m: &U) -> U {
            // ibig exposes modular arithmetic through a precomputed ModuloRing;
            // build it here so the full one-shot cost is measured.
            let ring = ModuloRing::new(m);
            ring.from(a.clone()).pow(exp).residue()
        }
    }
}

// ---- num-bigint impls -----------------------------------------------------

pub use num_backend::Num;

mod num_backend {
    use super::{
        ibig_sign_hex, sample_ibig, sample_ubig, ubig_hex, Backend, BenchInt, PrimitiveBackend,
        PrimitiveInt, SignedInt, UnsignedInt, ValueClass,
    };
    use core::fmt::Write as _;
    use num_bigint::{BigInt, BigUint};
    use num_traits::{Num as _, ToPrimitive as _};
    use rand_v08::Rng;

    fn to_u(u: &super::UBig) -> BigUint {
        BigUint::from_str_radix(&ubig_hex(u), 16).unwrap()
    }
    fn to_s(i: super::IBig) -> BigInt {
        let (neg, hex) = ibig_sign_hex(i);
        let mag = BigInt::from(BigUint::from_str_radix(&hex, 16).unwrap());
        if neg {
            -mag
        } else {
            mag
        }
    }

    impl BenchInt for BigUint {
        fn parse(s: &str) -> Self {
            s.parse().unwrap()
        }
        fn add_ref(&self, rhs: &Self) -> Self {
            self + rhs
        }
        fn sub_ref(&self, rhs: &Self) -> Self {
            self - rhs
        }
        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
        fn div_ref(&self, rhs: &Self) -> Self {
            self / rhs
        }
        fn bitand_ref(&self, rhs: &Self) -> Self {
            self & rhs
        }
        fn bitxor_ref(&self, rhs: &Self) -> Self {
            self ^ rhs
        }
        fn shl_ref(&self, bits: usize) -> Self {
            self << bits
        }
        fn shr_ref(&self, bits: usize) -> Self {
            self >> bits
        }
        fn add_assign_ref(&mut self, rhs: &Self) {
            *self += rhs;
        }
        fn sub_assign_ref(&mut self, rhs: &Self) {
            *self -= rhs;
        }
        fn bitxor_assign_ref(&mut self, rhs: &Self) {
            *self ^= rhs;
        }
    }

    impl UnsignedInt for BigUint {
        fn from_u64(v: u64) -> Self {
            BigUint::from(v)
        }
        fn from_u128(v: u128) -> Self {
            BigUint::from(v)
        }
    }

    impl BenchInt for BigInt {
        fn parse(s: &str) -> Self {
            s.parse().unwrap()
        }
        fn add_ref(&self, rhs: &Self) -> Self {
            self + rhs
        }
        fn sub_ref(&self, rhs: &Self) -> Self {
            self - rhs
        }
        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
        fn div_ref(&self, rhs: &Self) -> Self {
            self / rhs
        }
        fn bitand_ref(&self, rhs: &Self) -> Self {
            self & rhs
        }
        fn bitxor_ref(&self, rhs: &Self) -> Self {
            self ^ rhs
        }
        fn shl_ref(&self, bits: usize) -> Self {
            self << bits
        }
        fn shr_ref(&self, bits: usize) -> Self {
            self >> bits
        }
        fn add_assign_ref(&mut self, rhs: &Self) {
            *self += rhs;
        }
        fn sub_assign_ref(&mut self, rhs: &Self) {
            *self -= rhs;
        }
        fn bitxor_assign_ref(&mut self, rhs: &Self) {
            *self ^= rhs;
        }
    }

    impl SignedInt for BigInt {
        fn from_i64(v: i64) -> Self {
            BigInt::from(v)
        }
        fn from_i128(v: i128) -> Self {
            BigInt::from(v)
        }
        fn from_u64(v: u64) -> Self {
            BigInt::from(v)
        }
        fn from_u128(v: u128) -> Self {
            BigInt::from(v)
        }
        fn try_to_i128(&self) -> Option<i128> {
            self.to_i128()
        }
    }

    impl PrimitiveInt for BigUint {
        fn to_radix_string(&self, radix: u32) -> String {
            self.to_str_radix(radix)
        }
        fn from_radix(s: &str, radix: u32) -> Self {
            BigUint::from_str_radix(s, radix).unwrap()
        }
        fn write_hex(&self, out: &mut String) {
            write!(out, "{:x}", self).unwrap();
        }
        fn pow_exp(&self, exp: usize) -> Self {
            self.pow(exp as u32)
        }
        fn gcd(&self, rhs: &Self) -> Self {
            num_integer::Integer::gcd(self, rhs)
        }
        fn gcd_ext_blackbox(&self, rhs: &Self) {
            // BigUint has no signed cofactors; do the extended gcd on BigInt.
            let a = BigInt::from(self.clone());
            let b = BigInt::from(rhs.clone());
            core::hint::black_box(num_integer::Integer::extended_gcd(&a, &b));
        }
    }

    /// num-bigint backend: pure-Rust `BigUint` / `BigInt`.
    pub struct Num;

    impl Backend for Num {
        const NAME: &'static str = "num";
        type Unsigned = BigUint;
        type Signed = BigInt;

        fn sample_unsigned<R: Rng>(class: ValueClass, rng: &mut R) -> BigUint {
            to_u(&sample_ubig(class, rng))
        }
        fn sample_signed<R: Rng>(class: ValueClass, rng: &mut R) -> BigInt {
            to_s(sample_ibig(class, rng))
        }
        fn magnitude(value: BigInt) -> BigUint {
            value.into_parts().1
        }
        fn unsigned_to_signed(value: BigUint) -> BigInt {
            BigInt::from(value)
        }
    }

    impl PrimitiveBackend for Num {
        fn sample_unsigned_bits<R: Rng>(bits: usize, rng: &mut R) -> BigUint {
            to_u(&super::random_ubig(bits, rng))
        }
        fn mod_mul(a: &BigUint, b: &BigUint, m: &BigUint) -> BigUint {
            a * b % m
        }
        fn mod_pow(a: &BigUint, exp: &BigUint, m: &BigUint) -> BigUint {
            a.modpow(exp, m)
        }
    }
}

// ---- malachite impls ------------------------------------------------------

pub use malachite_backend::Malachite;

mod malachite_backend {
    use super::{
        ibig_sign_hex, sample_ibig, sample_ubig, ubig_hex, Backend, BenchInt, PrimitiveBackend,
        PrimitiveInt, SignedInt, UnsignedInt, ValueClass,
    };
    use malachite_base::num::arithmetic::traits::{ExtendedGcd, Gcd, ModPow, Pow, UnsignedAbs};
    use malachite_base::num::conversion::traits::{FromStringBase, ToStringBase};
    use malachite_nz::integer::Integer;
    use malachite_nz::natural::Natural;
    use rand_v08::Rng;

    fn to_u(u: &super::UBig) -> Natural {
        Natural::from_string_base(16, &ubig_hex(u)).unwrap()
    }
    fn to_s(i: super::IBig) -> Integer {
        let (neg, hex) = ibig_sign_hex(i);
        let mag = Integer::from(Natural::from_string_base(16, &hex).unwrap());
        if neg {
            -mag
        } else {
            mag
        }
    }

    impl BenchInt for Natural {
        fn parse(s: &str) -> Self {
            s.parse().unwrap()
        }
        fn add_ref(&self, rhs: &Self) -> Self {
            self + rhs
        }
        fn sub_ref(&self, rhs: &Self) -> Self {
            self - rhs
        }
        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
        fn div_ref(&self, rhs: &Self) -> Self {
            self / rhs
        }
        fn bitand_ref(&self, rhs: &Self) -> Self {
            self & rhs
        }
        fn bitxor_ref(&self, rhs: &Self) -> Self {
            self ^ rhs
        }
        fn shl_ref(&self, bits: usize) -> Self {
            self << bits
        }
        fn shr_ref(&self, bits: usize) -> Self {
            self >> bits
        }
    }

    impl UnsignedInt for Natural {
        fn from_u64(v: u64) -> Self {
            Natural::from(v)
        }
        fn from_u128(v: u128) -> Self {
            Natural::from(v)
        }
    }

    impl BenchInt for Integer {
        fn parse(s: &str) -> Self {
            s.parse().unwrap()
        }
        fn add_ref(&self, rhs: &Self) -> Self {
            self + rhs
        }
        fn sub_ref(&self, rhs: &Self) -> Self {
            self - rhs
        }
        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
        fn div_ref(&self, rhs: &Self) -> Self {
            self / rhs
        }
        fn bitand_ref(&self, rhs: &Self) -> Self {
            self & rhs
        }
        fn bitxor_ref(&self, rhs: &Self) -> Self {
            self ^ rhs
        }
        fn shl_ref(&self, bits: usize) -> Self {
            self << bits
        }
        fn shr_ref(&self, bits: usize) -> Self {
            self >> bits
        }
    }

    impl SignedInt for Integer {
        fn from_i64(v: i64) -> Self {
            Integer::from(v)
        }
        fn from_i128(v: i128) -> Self {
            Integer::from(v)
        }
        fn from_u64(v: u64) -> Self {
            Integer::from(v)
        }
        fn from_u128(v: u128) -> Self {
            Integer::from(v)
        }
        fn try_to_i128(&self) -> Option<i128> {
            i128::try_from(self).ok()
        }
    }

    impl PrimitiveInt for Natural {
        fn to_radix_string(&self, radix: u32) -> String {
            self.to_string_base(radix as u8)
        }
        fn from_radix(s: &str, radix: u32) -> Self {
            Natural::from_string_base(radix as u8, s).unwrap()
        }
        fn write_hex(&self, out: &mut String) {
            out.push_str(&self.to_string_base(16));
        }
        fn pow_exp(&self, exp: usize) -> Self {
            Pow::pow(self, exp as u64)
        }
        fn gcd(&self, rhs: &Self) -> Self {
            Gcd::gcd(self, rhs)
        }
        fn gcd_ext_blackbox(&self, rhs: &Self) {
            core::hint::black_box(ExtendedGcd::extended_gcd(self, rhs));
        }
    }

    /// malachite backend: pure-Rust `Natural` / `Integer`.
    pub struct Malachite;

    impl Backend for Malachite {
        const NAME: &'static str = "malachite";
        type Unsigned = Natural;
        type Signed = Integer;

        fn sample_unsigned<R: Rng>(class: ValueClass, rng: &mut R) -> Natural {
            to_u(&sample_ubig(class, rng))
        }
        fn sample_signed<R: Rng>(class: ValueClass, rng: &mut R) -> Integer {
            to_s(sample_ibig(class, rng))
        }
        fn magnitude(value: Integer) -> Natural {
            value.unsigned_abs()
        }
        fn unsigned_to_signed(value: Natural) -> Integer {
            Integer::from(value)
        }
    }

    impl PrimitiveBackend for Malachite {
        fn sample_unsigned_bits<R: Rng>(bits: usize, rng: &mut R) -> Natural {
            to_u(&super::random_ubig(bits, rng))
        }
        fn mod_mul(a: &Natural, b: &Natural, m: &Natural) -> Natural {
            a * b % m
        }
        fn mod_pow(a: &Natural, exp: &Natural, m: &Natural) -> Natural {
            // malachite's `mod_pow` requires the base already reduced; the
            // other backends reduce internally, so do the same here.
            let base = a % m;
            ModPow::mod_pow(&base, exp, m)
        }
    }
}
