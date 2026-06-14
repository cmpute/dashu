/// Machine word.
pub type Word = u32;

/// Signed machine word.
pub type SignedWord = i32;

/// Double machine word.
pub type DoubleWord = u64;

/// Signed double machine word.
pub type SignedDoubleWord = i64;

/// Accumulator for the product of three primes (3 × 2^32 ≈ 2^96).
#[derive(Clone, Copy, Debug, Default)]
pub struct TripleWord(pub [u32; 3]);
