# Cheatsheet

A dense reference for the dashu numeric types. See the linked pages for detail.

## Types

| Type | Crate | Description | Literal |
|------|-------|-------------|---------|
| `UBig` | dashu-int | unsigned integer | `ubig!(123)` |
| `IBig` | dashu-int | signed integer | `ibig!(-123)` |
| `FBig` | dashu-float | float, base 2 by default | `fbig!(0x1.8)` |
| `DBig` | dashu-float | decimal float, base 10 | `dbig!(1.5)` |
| `RBig` | dashu-ratio | rational | `rbig!(22/7)` |
| `CBig` | dashu-cmplx | complex, base 2 by default | `cbig!(1+2i)` |

## Construction

| Way | Example |
|-----|---------|
| `From` primitive | `UBig::from(123u32)` |
| parse | `"12.34".parse::<DBig>()?` |
| from parts | `RBig::from_parts(1.into(), 3u8.into())` |
| literal macro | `dbig!(1.5)`, `cbig!(1+2i)` |
| raw words | `UBig::from_words(&[3, 2, 1])` |

## Conversion

Lossless conversions use `From`; potentially-lossy ones use `TryFrom` (which fails on any precision loss). See [Conversion](./convert.md) for the full matrix.

| From → To | Trait | Notes |
|-----------|-------|-------|
| `UBig` → `IBig` | `From` | |
| `IBig` → `UBig` | `TryFrom` | fails if negative |
| int → `FBig` | `From` | precision inferred from magnitude |
| `FBig` → int | `TryFrom` | fails if fractional or infinite |
| `FBig` → `f32`/`f64` | `.to_f32()` / `.to_f64()` | returns `Rounded<f*>` |
| `f32`/`f64` → `FBig` | `TryFrom` | base 2 only |
| real → `CBig` | `From` | imaginary part `+0` |
| `CBig` → `FBig` | `TryFrom` | fails unless imaginary is zero |

## Operators

| Type | `+ - * /` | `%` | `<< >>` | `& \| ^ !` |
|------|:---:|:---:|:---:|:---:|
| `UBig` / `IBig` | ✓ | ✓ | ✓ | ✓ |
| `FBig` / `DBig` | ✓ | — | — | — |
| `RBig` | ✓ | — | — | — |
| `CBig` | ✓ | — | — | — |

## Formatting

| Type | `Display` | `Debug` | Other |
|------|-----------|---------|-------|
| `UBig`/`IBig` | decimal | head‥tail (+ digits/bits with `#?`) | `Binary`/`Octal`/`Hex`, `in_radix(2..=36)` |
| `FBig`/`DBig` | positional | `sig * base ^ exp` | `LowerExp`/`UpperExp` |
| `RBig` | `num/den` | — | `in_radix`, `in_expanded` |
| `CBig` | `a+bi` | `re:.. im:.. (prec: ..)` | — |

## Key methods

| Method | On | Returns |
|--------|-----|---------|
| `.exp()` / `.ln()` / `.sqrt()` | `FBig`, `CBig` | same type |
| `.sin()` / `.cos()` / `.tan()` / `.sin_cos()` | `FBig`, `CBig` | same type |
| `.powi(IBig)` / `.powf(&Self)` | `FBig`, `CBig` | same type |
| `.with_precision(p)` | `FBig` | `Rounded<FBig>` |
| `.to_decimal()` / `.to_binary()` | `FBig` | `Rounded<DBig>` / `Rounded<FBig>` |
| `.conj()` / `.proj()` | `CBig` | `CBig` |
| `.abs()` / `.arg()` / `.norm()` | `CBig` | `FBig` |
| `.gcd(&b)` / `.gcd_ext(&b)` | `UBig`/`IBig` (`Gcd`) | `Self` / `(gcd, x, y)` |
