use pyo3::prelude::*;

use dashu_int::{UBig, IBig};
use dashu_float::DBig;
use dashu_ratio::RBig;
type FBig = dashu_float::FBig;

/// This struct is used for representing [UBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "UBig")]
pub struct UPy(UBig);

impl From<UBig> for UPy {
    fn from(n: UBig) -> Self {
        UPy(n)
    }
}

/// This struct is used for representing [IBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "IBig")]
pub struct IPy(IBig);

impl From<IBig> for IPy {
    fn from(n: IBig) -> Self {
        IPy(n)
    }
}

/// This struct is used for representing [FBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "FBig")]
pub struct FPy(FBig);

impl From<FBig> for FPy {
    fn from(n: FBig) -> Self {
        FPy(n)
    }
}

/// This struct is used for representing [DBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "DBig")]
pub struct DPy(DBig);

impl From<DBig> for DPy {
    fn from(n: DBig) -> Self {
        DPy(n)
    }
}

/// This struct is used for representing [RBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "RBig")]
pub struct RPy(RBig);

impl From<RBig> for RPy {
    fn from(n: RBig) -> Self {
        RPy(n)
    }
}
