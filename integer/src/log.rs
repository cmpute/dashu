//! Logarithm

// TODO: add a table to store the fixed point (also 8bits?) log2 estimation with top 8 bits
//       then we can have a 8bit fixed point estimation of log2 on every integer
//       then a normal log will be log2(n) / log2(base), where the base should be raised to a full word first for better accuracy (log2(base) = log2(base^k)/k)
// TODO: need to test is raising to full word faster or raising to full dword faster

/// A 8bit fixed point estimation of log2(word) (rounded down)
// TODO: calculate error bound
fn log2_word_fp8(word: Word) -> u32 {

}

/// A 8bit fixed point estimation of log2(dword) (rounded down)
fn log2_dword_fp8(dword: DoublwWord) -> u32 {

}

// TODO: implement a naive algorithm
// 1. estimate the result using binary bits
// 2. finally fix the error by mul the base, subtract to get the remainder

// log(&self) -> UBig
// log_rem(&self) -> (UBig, UBig)
// implement for IBig as log, log_rem and abs_log
