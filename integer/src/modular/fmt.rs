//! Formatting modular rings and modular numbers.

use crate::modular::{
    modulo::{Modulo, ModuloRepr},
    modulo_ring::{
        ModuloRing, ModuloRingDouble, ModuloRingLarge, ModuloRingRepr, ModuloRingSingle,
    },
};
use core::fmt::{self, Binary, Debug, Display, Formatter, LowerHex, Octal, UpperHex};

macro_rules! impl_fmt {
    ($t:ident) => {
        impl $t for ModuloRing {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                match self.repr() {
                    ModuloRingRepr::Single(ring) => $t::fmt(ring, f),
                    ModuloRingRepr::Double(ring) => $t::fmt(ring, f),
                    ModuloRingRepr::Large(ring) => $t::fmt(ring, f),
                }
            }
        }

        impl $t for ModuloRingSingle {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("mod ")?;
                $t::fmt(&self.modulus(), f)
            }
        }

        impl $t for ModuloRingDouble {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("mod ")?;
                $t::fmt(&self.modulus(), f)
            }
        }

        impl $t for ModuloRingLarge {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                f.write_str("mod ")?;
                $t::fmt(&self.modulus(), f)
            }
        }

        impl $t for Modulo<'_> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                let residue = self.residue();
                $t::fmt(&residue, f)?;
                f.write_str(" (")?;
                match self.repr() {
                    ModuloRepr::Single(_, ring) => $t::fmt(ring, f)?,
                    ModuloRepr::Double(_, ring) => $t::fmt(ring, f)?,
                    ModuloRepr::Large(_, ring) => $t::fmt(ring, f)?,
                }
                f.write_str(")")
            }
        }
    };
}

impl_fmt!(Display);
impl_fmt!(Debug);
impl_fmt!(Binary);
impl_fmt!(Octal);
impl_fmt!(LowerHex);
impl_fmt!(UpperHex);
