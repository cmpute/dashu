//! Formatting modular rings and modular numbers.

use super::{
    modulo::{Modulo, ModuloRepr},
    modulo_ring::{
        ModuloRing, ModuloRingDouble, ModuloRingLarge, ModuloRingRepr, ModuloRingSingle,
    },
};
use core::fmt::{self, Binary, Debug, Display, Formatter, LowerHex, Octal, UpperHex};

macro_rules! impl_fmt_for_modulo_ring {
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
    };
}

macro_rules! impl_fmt_for_modulo {
    ($t:ident) => {
        impl_fmt_for_modulo_ring!($t);

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

impl_fmt_for_modulo!(Display);
impl_fmt_for_modulo!(Binary);
impl_fmt_for_modulo!(Octal);
impl_fmt_for_modulo!(LowerHex);
impl_fmt_for_modulo!(UpperHex);
impl_fmt_for_modulo_ring!(Debug);

impl Debug for Modulo<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let residue = self.residue();
        if f.alternate() {
            let modulus = match self.repr() {
                ModuloRepr::Single(_, ring) => ring.modulus(),
                ModuloRepr::Double(_, ring) => ring.modulus(),
                ModuloRepr::Large(_, ring) => ring.modulus(),
            };
            f.debug_struct("Modulo")
                .field("residue", &residue)
                .field("modulus", &modulus)
                .finish()
        } else {
            Debug::fmt(&residue, f)?;
            f.write_str(" (")?;
            match self.repr() {
                ModuloRepr::Single(_, ring) => Debug::fmt(ring, f)?,
                ModuloRepr::Double(_, ring) => Debug::fmt(ring, f)?,
                ModuloRepr::Large(_, ring) => Debug::fmt(ring, f)?,
            }
            f.write_str(")")
        }
    }
}
