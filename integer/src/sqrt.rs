// TODO: implement sqrt using "Karatsuba Sqrt"
// Ref: https://gmplib.org/manual/Square-Root-Algorithm

/*

For square root:

n = a*B2 + b
sqrt(n) = sa*B + r
n = a*B2 + 2sa*rB + r2

let a0 = sa*B
newton: a1 = 1/2 * (n/a0+a0) = 1/2 * (sa*B + 2r + r2/(sa*B) + sa*B)
= sa*B + r + r2/(sa*B)

Similarly for cubic root:

n = a*B3 + b
cbrt(n) = ca*B + r
n = a*B3 + 3ca^2*rB2 + 3ca*r^2B + r^3

let a0 = ca*B
newton: a1 = 1/3 * (n/a0^2+2a0) = 1/3 * (ca*B + 3r + 3r^2/ca*B + r^3/(ca*B)^2 + 2*ca*B)
= ca*B + r + r^2/(ca*B) + r^3/(ca*B)^2

*/