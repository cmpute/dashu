use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::ops::{Add, Div, Mul, Rem, Sub};

use dashu_base::Abs;
use dashu_float::round::mode;
use dashu_ratio::RBig;
use num_order::{NumHash, NumOrd};
use pyo3::{Bound, IntoPyObjectExt, Py, PyAny, PyResult, basic::CompareOp, intern, prelude::*};

use crate::{
    convert::{parse_error_to_py, parse_to_rbig},
    types::{DPy, FPy, IPy, PySign, RPy, UPy, UniInput},
};

/// Forward/reverse arithmetic dispatchers that coerce the operand to RBig first.
macro_rules! impl_rpy_binops {
    ($method:ident, $rs_method:ident) => {
        fn $method(lhs: &RPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let rhs = rhs.into_rpy()?;
            RPy((&lhs.0).$rs_method(&rhs.0)).into_py_any(py)
        }
    };
    ($method:ident, $rev_method:ident, $rs_method:ident) => {
        impl_rpy_binops!($method, $rs_method);
        fn $rev_method(lhs: UniInput<'_>, rhs: &RPy, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let lhs = lhs.into_rpy()?;
            RPy((&lhs.0).$rs_method(&rhs.0)).into_py_any(py)
        }
    };
}

impl_rpy_binops!(rpy_add, add);
impl_rpy_binops!(rpy_sub, rpy_rsub, sub);
impl_rpy_binops!(rpy_mul, mul);
impl_rpy_binops!(rpy_div, rpy_rdiv, div);
impl_rpy_binops!(rpy_mod, rpy_rmod, rem);

fn rpy_richcmp(lhs: &RPy, other: UniInput<'_>, op: CompareOp) -> bool {
    let order = match other {
        UniInput::Uint(x) => lhs.0.num_cmp(&x),
        UniInput::Int(x) => lhs.0.num_cmp(&x),
        UniInput::BUint(x) => lhs.0.num_cmp(&x.0),
        UniInput::BInt(x) => lhs.0.num_cmp(&x.0),
        UniInput::OBInt(x) => lhs.0.num_cmp(&x),
        UniInput::Float(x) => lhs.0.num_cmp(&x),
        UniInput::BFloat(x) => lhs.0.num_cmp(&x.0),
        UniInput::BDecimal(x) => lhs.0.num_cmp(&x.0),
        UniInput::OBDecimal(x) => lhs.0.num_cmp(&x),
        UniInput::BRational(x) => lhs.0.cmp(&x.0),
        UniInput::OBRational(x) => lhs.0.cmp(&x),
    };
    op.matches(order)
}

#[pymethods]
impl RPy {
    #[new]
    fn __new__(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(s) = ob.extract::<String>() {
            let r = RBig::from_str_with_radix_prefix(&s).map(|v| v.0);
            return Ok(RPy(r.map_err(parse_error_to_py)?));
        }
        // fractions.Fraction fast path (preserves the original parse_to_rbig round-trip)
        {
            let py = ob.py();
            let fractions = py.import(intern!(py, "fractions"))?;
            let fraction_type = fractions.getattr(intern!(py, "Fraction"))?;
            if ob.is_instance(&fraction_type)? {
                return Ok(RPy(parse_to_rbig(ob)?));
            }
        }
        if let Ok(obj) = ob.extract::<PyRef<Self>>() {
            return Ok(RPy(obj.0.clone()));
        }
        // any other Python number -> permissive construction
        Ok(RPy(UniInput::extract(ob.as_borrowed())?.construct_rpy()?))
    }
    fn unwrap(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        crate::convert::convert_from_rbig(&self.0, py)?.into_py_any(py)
    }

    fn __repr__(&self) -> String {
        format!("<RBig {:?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __format__(&self, _format_spec: &str) -> String {
        format!("{}", self.0)
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.num_hash(&mut hasher);
        hasher.finish()
    }
    fn __richcmp__(&self, other: UniInput<'_>, op: CompareOp) -> bool {
        rpy_richcmp(self, other, op)
    }
    fn __bool__(&self) -> bool {
        !self.0.is_zero()
    }
    fn __float__(&self) -> f64 {
        self.0.to_f64_fast()
    }
    fn __int__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        crate::convert::convert_from_ibig(&self.0.trunc(), py)?.into_py_any(py)
    }

    /********** arithmetic **********/
    #[inline]
    fn __add__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_add(self, other, py)
    }
    #[inline]
    fn __radd__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_add(self, other, py)
    }
    #[inline]
    fn __sub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_sub(self, other, py)
    }
    #[inline]
    fn __rsub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_rsub(other, self, py)
    }
    #[inline]
    fn __mul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_mul(self, other, py)
    }
    #[inline]
    fn __rmul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_mul(self, other, py)
    }
    #[inline]
    fn __truediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_div(self, other, py)
    }
    #[inline]
    fn __rtruediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_rdiv(other, self, py)
    }
    #[inline]
    fn __mod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_mod(self, other, py)
    }
    #[inline]
    fn __rmod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_rmod(other, self, py)
    }
    #[inline]
    fn __neg__(&self) -> Self {
        RPy(-&self.0)
    }
    #[inline]
    fn __pos__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    #[inline]
    fn __abs__(&self) -> Self {
        RPy(self.0.clone().abs())
    }

    /********** properties & predicates **********/
    #[getter]
    fn numerator(&self) -> IPy {
        IPy(self.0.numerator().clone())
    }
    #[getter]
    fn denominator(&self) -> UPy {
        UPy(self.0.denominator().clone())
    }
    fn is_int(&self) -> bool {
        self.0.is_int()
    }
    fn is_one(&self) -> bool {
        self.0.is_one()
    }
    fn sign(&self) -> PySign {
        self.0.sign().into()
    }
    fn signum(&self) -> Self {
        RPy(self.0.signum())
    }

    /********** rounding (return IBig / RPy) **********/
    fn trunc(&self) -> IPy {
        IPy(self.0.trunc())
    }
    fn floor(&self) -> IPy {
        IPy(self.0.floor())
    }
    fn ceil(&self) -> IPy {
        IPy(self.0.ceil())
    }
    fn round(&self) -> IPy {
        IPy(self.0.round())
    }
    fn fract(&self) -> Self {
        RPy(self.0.fract())
    }
    fn split_at_point(&self) -> (IPy, Self) {
        let (int_part, frac_part) = self.0.clone().split_at_point();
        (IPy(int_part), RPy(frac_part))
    }

    /********** powers **********/
    fn sqr(&self) -> Self {
        RPy(self.0.sqr())
    }
    fn cubic(&self) -> Self {
        RPy(self.0.cubic())
    }
    fn pow(&self, n: usize) -> Self {
        RPy(self.0.pow(n))
    }

    /********** construction & conversion **********/
    fn to_int(&self) -> IPy {
        IPy(self.0.trunc())
    }
    /// Lossy conversion to a binary float with the given precision.
    fn to_float(&self, precision: usize) -> FPy {
        FPy(self.0.to_float::<mode::Zero, 2>(precision).value())
    }
    /// Lossy conversion to a decimal float with the given precision.
    fn to_decimal(&self, precision: usize) -> DPy {
        DPy(self.0.to_float::<mode::HalfAway, 10>(precision).value())
    }
    #[staticmethod]
    fn from_parts(numerator: &IPy, denominator: &UPy) -> Self {
        RPy(RBig::from_parts(numerator.0.clone(), denominator.0.clone()))
    }
    /// Find the simplest rational within the error bounds of the given float.
    #[staticmethod]
    fn simplest_from_float(f: &FPy) -> Option<Self> {
        RBig::simplest_from_float(&f.0).map(RPy)
    }
}
