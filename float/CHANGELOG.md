# Changelog

## 0.2.0 (Initial release)

- Support basic arithmetic operations (`add`/`sub`/`mul`/`div`/`exp`/`ln`) and base conversion.

# Todo

## Roadmap to next version
- Support generating base math constants (E, Pi, SQRT2, etc.)
- Implement Random generator
- Implement Serde serialization
- Implememt sqrt, cbrt, nth_root
- Implement powf, log
- Create operations benchmark
- Benchmark against crates: rug, twofloat, num-bigfloat, rust_decimal, bigdecimal, scientific
- Implement more formatting traits

## Roadmap to v1.0
- Determine if caches for constants (especially ln2, pi) should be stored in the context (using RC)

## Not in plan for v1.0
- Other math functions: sin/cos/tan/etc.
- Support more rounding modes
- Faster base conversion (references: [dragonbox](https://github.com/jk-jeon/dragonbox), [ryu](https://lib.rs/crates/ryu-js), [Articles by Lemire](https://arxiv.org/search/cs?searchtype=author&query=Lemire%2C+D), [Fast number parsing by Lemire](https://arxiv.org/pdf/2101.11408.pdf)
- Specialize algorithms in the range where IBig is inlined

