# Roadmap to v0.1
- [ ] Add complete tests
- [ ] Implement macros for creating float from literals
- [x] Add methods to return inexactness
- [ ] Implement +-*/ and exp/log
- [ ] Implement more arithmetic traits with primitive types
- [ ] Implement more arithmetic traits for reference type

# Roadmap to v0.2
- [ ] Implement *assign traits
- [ ] Implement Random generator
- [ ] Implement Serde serialization

# Roadmap to v1.0
- [ ] Upstream certain math operations to IBig
- [ ] Implement basic arithmetics with correct rounding (see [https://www.mpfr.org/algorithms.pdf])
- [ ] Implement sqrt, rec_sqrt (reciprocal of square roots), cbrt, nth_root (maybe exp/ln)
- [ ] fast float printing (references: [dragonbox](https://github.com/jk-jeon/dragonbox), [ryu](https://lib.rs/crates/ryu-js), [Articles by Lemire](https://arxiv.org/search/cs?searchtype=author&query=Lemire%2C+D), [Fast number parsing by Lemire](https://arxiv.org/pdf/2101.11408.pdf), specialize algorithms in the range where IBig is inlined

# Not in plan for v1.0
- [ ] other primitive math functions: sin/cos/tan
- [ ] Support more rounding modes
