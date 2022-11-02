Contents of the user guide:
- Type introduction
  - Basic Types (`Sign`, `Approximation`)
  - Memory Layout
- Construction and destruction
  - Raw constructor (`from_words` and `as_words` for `UBig`)
  - Parts `from_parts` and `into_parts` (`into_repr` for `FBig`)
  - Const constructor `from_parts_const` (`from_word` for `UBig`)
- Conversion (clarify fallible or infallible)
  - Conversion between types
  - Conversion between primitives
- Parsing and Formatting
  - Standard Parsing API `from_str`, `from_str_radix`
  - Standard Formatting Traits
  - Debug Print
  - Float number parsing
  - Rational number formatting
  