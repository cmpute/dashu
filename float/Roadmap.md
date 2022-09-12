# Roadmap to v0.1
- [ ] Implement macros for creating float from literals
- [x] Add methods to return inexactness
- [x] Implement +-*/ and exp/log
- [x] Implement more arithmetic traits with primitive types
- [x] Implement more arithmetic traits for reference type
- [x] Full support and tests for inf
- [x] Full support and tests for arbitrary precision (precision = 0)
- [x] Make conversion between bases ready
- [ ] Full documentation

# Roadmap to v0.2
- [ ] Support generating base math constants (E, Pi, SQRT2, etc.)
- [ ] Implement Random generator
- [ ] Implement Serde serialization
- [ ] Implememt sqrt, cbrt, nth_root
- [ ] Implement powf, log
- [ ] Benchmark against crates: twofloat, num-bigfloat, rust_decimal, bigdecimal, scientific
- [ ] Implement more formatting traits

# Roadmap to v1.0
- [ ] fast float printing (references: [dragonbox](https://github.com/jk-jeon/dragonbox), [ryu](https://lib.rs/crates/ryu-js), [Articles by Lemire](https://arxiv.org/search/cs?searchtype=author&query=Lemire%2C+D), [Fast number parsing by Lemire](https://arxiv.org/pdf/2101.11408.pdf), specialize algorithms in the range where IBig is inlined
- [ ] Determine if caches for constants (especially ln2, pi) should be stored in the context (using RC)

# Not in plan for v1.0
- [ ] other primitive math functions: sin/cos/tan
- [ ] Support more rounding modes
