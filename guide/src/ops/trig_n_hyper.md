`FBig`/`DBig` and `CBig` provide the trigonometric and hyperbolic functions. They are grouped on one page because the complex circular functions are built from the real circular *and* hyperbolic functions.

## Real functions

- Circular: `sin`, `cos`, `tan`, and `sin_cos` (both at once); inverses `asin`, `acos`, `atan`, and the four-quadrant `atan2(y, x)`.
- ×u variants: `sin_unit(u)`, `cos_unit(u)`, `tan_unit(u)`, and `sin_cos_unit(u)` — of `2π·x/u`, the argument in units of the full turn divided by `u`, with the inverses `asin_unit(u)`, `acos_unit(u)`, `atan_unit(u)`, and the four-quadrant `atan2_unit`, called as `y.atan2_unit(x, u)` (the receiver is the y-coordinate, the first argument the x-coordinate). `u = 2` gives the ×π functions (`sin_pi`, …); `u = 360` gives degrees.
- Hyperbolic: `sinh`, `cosh`, `tanh`, `sinh_cosh`; inverses `asinh`, `acosh`, `atanh`. The ×π variants `sinh_pi`, `cosh_pi`, and `sinh_cosh_pi` use the shared cached π.

Angles are in radians. `atan2` follows the C99 signed-zero model, which matters for correct branch-cut behavior on the axes.

The ×u circular functions reduce the argument *exactly* mod u in integer arithmetic, so — unlike the radian functions — their accuracy does not degrade as the input grows (`sin_unit(10^100, 4)` is exactly `0`: `10^100` is a multiple of `4`). Arguments where `12x/u` (resp. `8x/u` for the tangent) is an integer resolve exactly: quarters and eighths to `0`/`±1` (`sin_unit(90, 360) == 1`), the sixths to `±1/2` (`sin_unit(30, 360) == 0.5`). At the odd multiples of `u/4` the tangent hits its poles, where the one-sided limits are `+∞` and `−∞`: the case is indeterminate and reported as an error. The inverse family returns exact `k·u/8` values on the axes and diagonals (`(1).atan2_unit(&(1), 360) == 45`), and the `u → 0` limit of every inverse (in domain) is the signed zero.

## Complex functions

`CBig` provides the circular family `sin`, `cos`, `tan`, `sin_cos`, `asin`, `acos`, and `atan`, the hyperbolic family `sinh`, `cosh`, `tanh`, `sinh_cosh`, `asinh`, `acosh`, and `atanh`, and the ×π circular family `sin_pi`, `cos_pi`, `tan_pi`, and `sin_cos_pi`. The circular functions are evaluated from the real `sin`/`cos` and `sinh`/`cosh` via the identities

$$\sin(x+iy) = \sin x\cosh y + i\cos x\sinh y, \qquad \cos(x+iy) = \cos x\cosh y - i\sin x\sinh y.$$

The ×π family composes the real `sin_cos_pi` with the hyperbolic `sinh_cosh_pi` of the scaled argument. The hyperbolic functions reuse the circular ones through the rotation identities `sinh z = -i\sin(iz)`, `cosh z = \cos(iz)`, and `tanh z = -i\tan(iz)` — an exact swap of the real and imaginary parts, so the two families share their rounding certification. The inverse functions (circular and hyperbolic) follow the Kahan signed-zero branch-cut formulation. See [Standards Compliance](../compliance.md) for the Annex G special-value and branch-cut tables.
