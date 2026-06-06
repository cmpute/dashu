## dashu-int Improvements

N/A

## dashu-ratio Improvements

- GCD: An idea of fast gcd check for rational number: don't do gcd reduction after every operation.
  For small numerators or denominators, we can directly do a gcd, otherwise, we first do gcd with a primorial that
  fits in a word, and only remove these small divisors.
  Further improvement: store a const divisor for the prime factors in the primorial, thus supports a fast factorial of
  the gcd result, and then divide with these const divisor.
