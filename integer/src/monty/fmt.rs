//! Formatting of Montgomery-form values.

use super::repr::Montgomery;
use core::fmt::{self, Binary, Debug, Display, Formatter, LowerHex, Octal, UpperHex};

macro_rules! impl_fmt_for_monty {
    ($t:ident) => {
        impl $t for Montgomery<'_> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                $t::fmt(&self.residue(), f)?;
                f.write_str(" (mod ")?;
                $t::fmt(&self.modulus(), f)?;
                f.write_str(")")
            }
        }
    };
}

impl_fmt_for_monty!(Display);
impl_fmt_for_monty!(Binary);
impl_fmt_for_monty!(Octal);
impl_fmt_for_monty!(LowerHex);
impl_fmt_for_monty!(UpperHex);

impl Debug for Montgomery<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let residue = self.residue();
        let modulus = self.modulus();
        if f.alternate() {
            f.debug_struct("Montgomery")
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
