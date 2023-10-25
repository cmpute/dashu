//! Formatting modular rings and modular numbers.

use super::repr::Reduced;
use core::fmt::{self, Binary, Debug, Display, Formatter, LowerHex, Octal, UpperHex};

macro_rules! impl_fmt_for_modulo {
    ($t:ident) => {
        impl $t for Reduced<'_> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                $t::fmt(&self.residue(), f)?;
                f.write_str(" (mod ")?;
                $t::fmt(&self.modulus(), f)?;
                f.write_str(")")
            }
        }
    };
}

impl_fmt_for_modulo!(Display);
impl_fmt_for_modulo!(Binary);
impl_fmt_for_modulo!(Octal);
impl_fmt_for_modulo!(LowerHex);
impl_fmt_for_modulo!(UpperHex);

impl Debug for Reduced<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let residue = self.residue();
        let modulus = self.modulus();
        if f.alternate() {
            f.debug_struct("Reduced")
                .field("residue", &residue)
                .field("modulus", &modulus)
                .finish()
        } else {
            Debug::fmt(&residue, f)?;
            f.write_str(" (mod ")?;
            Debug::fmt(&self.modulus(), f)?;
            f.write_str(")")
        }
    }
}
