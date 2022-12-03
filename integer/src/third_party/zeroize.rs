use zeroize::Zeroize;

use crate::{
    buffer::Buffer,
    repr::{Repr, TypedRepr},
    IBig, UBig,
};

impl Zeroize for Buffer {
    fn zeroize(&mut self) {
        self.as_full_slice().zeroize();
        self.truncate(0)
    }
}

impl Zeroize for Repr {
    fn zeroize(&mut self) {
        self.as_full_slice().zeroize();
        self.clone_from(&Repr::zero());
    }
}

impl Zeroize for TypedRepr {
    #[inline]
    fn zeroize(&mut self) {
        if let TypedRepr::Large(buffer) = self {
            buffer.zeroize()
        }
        *self = TypedRepr::Small(0)
    }
}

impl Zeroize for UBig {
    #[inline]
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}

impl Zeroize for IBig {
    #[inline]
    fn zeroize(&mut self) {
        self.0.zeroize()
    }
}
