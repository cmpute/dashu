# Roadmap to v0.1

- [ ] Remove AndNot from public API as it's not widely used
- [ ] Remove operations for UBig with signed primitive integers (because the Output is still unsigned, which makes no sense).
- [ ] Add operations between UBig and IBig (the result should be IBig)
- [ ] Format code and inspect inline/pub(crate) markers
- [ ] Add more tests to cover cases with inline double words
