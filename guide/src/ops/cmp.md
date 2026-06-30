# Equality and Comparison

Comparison is natively enabled **only between big numbers of the same kind**, not between big numbers and primitive types — this avoids the trait-overlap problem described in [`num-bigint`#150](https://github.com/rust-num/num-bigint/issues/150). To compare a big number with a primitive type, enable the `num-order` feature and use the `NumOrd` trait.

## Equality

`PartialEq`/`Eq` is value equality. For `FBig`/`DBig` it compares the representation and ignores the context (precision and rounding mode), so two floats with different precision but the same value compare equal. Signed zeros compare equal: `+0 == -0`. `CBig` compares componentwise, with `+0 == -0` on each part.

## Ordering

`UBig`/`IBig`/`RBig`/`FBig`/`DBig` carry the natural numeric total order (`Ord`). Infinities are placed at the ends: $-\infty < \text{finite} < +\infty$. `CBig` defines a lexicographic total order by `(re, then im)` — usable for sorting and `BTreeMap`, but note it is *not* an algebraic magnitude ordering.

## Sign

The signed types (`IBig`, `FBig`/`DBig`, `RBig`, `CBig`) expose `.sign()` (returning `dashu_base::Sign`, where zero is `Positive`) and `.signum()` (returning `-1`, `0`, or `+1` as the same type).

## Magnitude comparison and cross-type ordering

`AbsOrd` (from `dashu-base`) compares by absolute value; for `CBig` it compares by $|z|$. The `num-order` feature adds `NumOrd` for ordering and `NumHash` for hashing across different numeric types (big and primitive), keeping them consistent with each other.
