//! Macros forwarding operator traits to the [`CBig`] context-layer operations.
//!
//! Following `FBig`, binary operators with a trait (`Add`/`Sub`/`Mul`/`Div`) and unary `Neg` have
//! **no** inherent method on [`CBig`] — the operator *is* the convenience API, and it computes the
//! result context (`max(lhs, rhs)`), calls the context-layer op, and unwraps via [`Context::unwrap_cfp`].
//! The identifiers used inside the macro (`CBig`, `Context`, `Round`, `Word`) resolve at the call
//! site, so call sites must keep them in scope.
//!
//! [`CBig`]: crate::cbig::CBig
//! [`Context::unwrap_cfp`]: crate::repr::Context::unwrap_cfp

/// Implement a binary operator (`Add`/`Sub`/`Mul`/`Div`) and its `Assign` form for all four
/// ref/val combinations. Each forwards to `Context::$method` at `max(lhs, rhs)` precision.
macro_rules! impl_cbig_binop {
    ($trait:ident, $method:ident, $assign_trait:ident, $assign_method:ident) => {
        impl<R: Round, const B: Word> $trait for CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn $method(self, rhs: CBig<R, B>) -> Self::Output {
                let ctx = Context::max(self.context(), rhs.context());
                ctx.unwrap_cfp(ctx.$method(&self, &rhs))
            }
        }

        impl<R: Round, const B: Word> $trait<&CBig<R, B>> for CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn $method(self, rhs: &CBig<R, B>) -> Self::Output {
                let ctx = Context::max(self.context(), rhs.context());
                ctx.unwrap_cfp(ctx.$method(&self, rhs))
            }
        }

        impl<R: Round, const B: Word> $trait<CBig<R, B>> for &CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn $method(self, rhs: CBig<R, B>) -> Self::Output {
                let ctx = Context::max(self.context(), rhs.context());
                ctx.unwrap_cfp(ctx.$method(self, &rhs))
            }
        }

        impl<R: Round, const B: Word> $trait<&CBig<R, B>> for &CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn $method(self, rhs: &CBig<R, B>) -> Self::Output {
                let ctx = Context::max(self.context(), rhs.context());
                ctx.unwrap_cfp(ctx.$method(self, rhs))
            }
        }

        impl<R: Round, const B: Word> $assign_trait for CBig<R, B> {
            #[inline]
            fn $assign_method(&mut self, rhs: CBig<R, B>) {
                let ctx = Context::max(self.context(), rhs.context());
                *self = ctx.unwrap_cfp(ctx.$method(self, &rhs));
            }
        }

        impl<R: Round, const B: Word> $assign_trait<&CBig<R, B>> for CBig<R, B> {
            #[inline]
            fn $assign_method(&mut self, rhs: &CBig<R, B>) {
                let ctx = Context::max(self.context(), rhs.context());
                *self = ctx.unwrap_cfp(ctx.$method(self, rhs));
            }
        }
    };
}

/// Implement the four ref/val mixed-type operators `CBig op FBig` — each forwarding directly to
/// `Context::$ctx_method(self, rhs)`. The trait method name is `$trait_method`. Used for
/// `CBig * FBig` (componentwise, trait=multiply/mul, ctx=mul_real) and `CBig / FBig`
/// (componentwise, trait=divide/div, ctx=div_real). The identifiers (`CBig`, `FBig`, `Context`,
/// `Round`, `Word`) must be in scope at the call site.
macro_rules! impl_cbig_scalar_binop {
    ($op:ident, $trait_method:ident, $ctx_method:ident) => {
        impl<R: Round, const B: Word> $op<&FBig<R, B>> for &CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn $trait_method(self, rhs: &FBig<R, B>) -> CBig<R, B> {
                let ctx = Context::max(self.context(), Context(rhs.context()));
                ctx.unwrap_cfp(ctx.$ctx_method(self, rhs))
            }
        }
        impl<R: Round, const B: Word> $op<FBig<R, B>> for &CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn $trait_method(self, rhs: FBig<R, B>) -> CBig<R, B> {
                let ctx = Context::max(self.context(), Context(rhs.context()));
                ctx.unwrap_cfp(ctx.$ctx_method(self, &rhs))
            }
        }
        impl<R: Round, const B: Word> $op<&FBig<R, B>> for CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn $trait_method(self, rhs: &FBig<R, B>) -> CBig<R, B> {
                let ctx = Context::max(self.context(), Context(rhs.context()));
                ctx.unwrap_cfp(ctx.$ctx_method(&self, rhs))
            }
        }
        impl<R: Round, const B: Word> $op<FBig<R, B>> for CBig<R, B> {
            type Output = CBig<R, B>;
            #[inline]
            fn $trait_method(self, rhs: FBig<R, B>) -> CBig<R, B> {
                let ctx = Context::max(self.context(), Context(rhs.context()));
                ctx.unwrap_cfp(ctx.$ctx_method(&self, &rhs))
            }
        }
    };
}

pub(crate) use impl_cbig_binop;
pub(crate) use impl_cbig_scalar_binop;
