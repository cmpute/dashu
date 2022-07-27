pub mod base {
    pub use dashu_base::*;
}

pub mod integer {
    pub use dashu_int::*;
}

/// A verbose alias for [UBig][dashu_int::UBig]
pub type Natural = dashu_int::UBig;

/// A verbose alias for [IBig][dashu_int::IBig]
pub type Integer = dashu_int::IBig;
