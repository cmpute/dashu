//! Implementations of advanced math functions

// TODO: implement the math functions as associated methods, and add them to FBig through a trait
// REF: https://pkg.go.dev/github.com/ericlagergren/decimal

enum FpResult {
    Normal(Repr),
    Overflow,
    Underflow,
    NaN,

    /// An exact infinite result is obtained from finite inputs, such as
    /// divide by zero, logarithm on zero.
    Infinite,
}

impl Context {
    fn sin(&self, repr: Repr) -> FpResult {
        todo!()
    }
}

trait ContextOps {
    fn context(&self) -> &Context;
    fn repr(&self) -> &Repr;

    #[inline]
    fn sin(&self) -> FpResult {
        self.context().sin(self.repr())
    }
}