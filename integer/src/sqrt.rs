// TODO: implement sqrt using "Karatsuba Sqrt"
// Ref: https://gmplib.org/manual/Square-Root-Algorithm

/*

For square root:

n = a*B2 + b, b = b1*B + b0
let (sa, ra) = sqrt_rem(a) i.e a = sa^2 + ra
n = sa^2*B2 + ra*B2 + b1*B + b0
let initial guess s0 = sa*B, remainder r0 = sqrt(n) - s0
then n = (s0 + r0)^2 = (a-ra)*B2 + 2*s0*r0 + r0^2
so consider 2*s0*r0 - ra*B2 = 2*sa*r0*B - ra*B2 = b1*B => r0 = (b1 + ra*B) / 2*sa
let (q, r) = (b1 + ra*B) / 2*sa
n = sa^2*B2 + (2*sa*q + r)*B + b0
  = (sa*B + q)^2* - q^2 + r*B + b0 
second guess s1 = sa*B + q, remaining term r*B + b0 - q^2

think of newton: s1 = 1/2 * (n/s0+s0) = 1/2 * (s0 + 2r0 + r0^2/s0 + s0)
= s0 + r0 + r0^2/2s0
notice b = 2*s0*r0 + r0^2
so s1 = s0 + b / (2*s0) = s0 + b1 / 2*sa

Similarly for cubic root:

n = a*B3 + b, b = b2*B2 + b1*B + b0
let (ca, ra) = cbrt_rem(a) i.e a = ca^3 + ra
n = ca^3*B3 + ra*B3 + b2*B2 + b1*B + b0
let initial guess c0 = ca*B, remainder r0 = cbrt(n) - c0
then n = (c0 + r0)^3 = (a-ra)*B3 + 3c0^2*r0 + 3c0*r0^2 + r0^3
so consider 3c0^2*r0 - ra*B3 = 3ca^2*r0*B2 - ra*B3 = b2*B2 => r0 = (b2 + ra*B) / (3ca^2)

let (q, r) = (b2 + ra*B) / (3ca^2)
then n = ca^3*B3 + (3ca^2*q + r)*B2 + b1*B + b0
       = (ca*B + q)^3 - 3ca*q^2*B - q^3 + r*B2 + b1*B + b0

// let (q, r) = (b1 + b2*B + ra*B2) / (3ca^2)
// then n = ca^3*B3 + (3ca^2*q + r)*B2 + b0
//        = (ca*B + q)^3 - 3ca*q^2*B + r*B2 + b0
// second guess c1 = ca*B + q

// let (q, r) = (b2 + ra*B) / (3ca^2)
// then n = ca^3*B3 + (3ca^2*q + r)*B2 + b1*B + b0
//        = (ca*B + q)^3 - 3ca*q^2*B + r*B2 + b1*B + b0
// second guess c1 = ca*B + q, remainder r1 = cbrt(n) - c1
// (c1 + r1)^3 = c1^3 + 3c1^2*r1 + 3c1*r1^2 + r1^3
// = (a-ra)*B3 + 3ca^2*q*B2 + 3ca*q^2*B + q^3
//   + 3(ca^2*B2 + 2*ca*q*B + q^2)*r1
//   + 3(ca*B + q)*r1^2
//   + r1^3
// note that 3ca^2*q*B2 = (b2 + ra*B)*B2
// so consider 3ca*q^2*B + 3ca^2*r1*B2 + 6*ca*q*B + 3ca*r1^2*B = b1 * B
// r1 = (b1 - 3ca*q^2 - 6*ca*q) / ??
//
// let (q', r') = (r*B + b1 - 3ca*q^2) / (3ca) ??

*/