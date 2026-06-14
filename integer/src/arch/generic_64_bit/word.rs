/// Machine word.
pub type Word = u64;

/// Signed machine word.
pub type SignedWord = i64;

/// Double machine word.
pub type DoubleWord = u128;

/// Signed double machine word.
pub type SignedDoubleWord = i128;

/// Accumulator for the product of three primes (3 × 2^64 = 2^192).
#[derive(Clone, Copy, Debug, Default)]
pub struct TripleWord(pub [u64; 3]);
