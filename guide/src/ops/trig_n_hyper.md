# Trigonometric and Hyperbolic Functions

`FBig`/`DBig` and `CBig` provide the trigonometric and hyperbolic functions. They are grouped on one page because the complex circular functions are built from the real circular *and* hyperbolic functions.

## Real functions

- Circular: `sin`, `cos`, `tan`, and `sin_cos` (both at once); inverses `asin`, `acos`, `atan`, and the four-quadrant `atan2(y, x)`.
- Hyperbolic: `sinh`, `cosh`, `tanh`, `sinh_cosh`; inverses `asinh`, `acosh`, `atanh`.

Angles are in radians. `atan2` follows the C99 signed-zero model, which matters for correct branch-cut behavior on the axes.

## Complex functions

`CBig` provides the circular family `sin`, `cos`, `tan`, `sin_cos`, `asin`, `acos`, and `atan`. They are evaluated from the real `sin`/`cos` and `sinh`/`cosh` via the identities

$$\sin(x+iy) = \sin x\cosh y + i\cos x\sinh y, \qquad \cos(x+iy) = \cos x\cosh y - i\sin x\sinh y.$$

The inverse functions follow the Kahan signed-zero branch-cut formulation. (Complex-valued hyperbolic functions — `CBig::sinh`, `cosh`, … — are deferred to a later 0.5.x release.) See [Standards Compliance](../compliance.md) for the Annex G special-value and branch-cut tables.
