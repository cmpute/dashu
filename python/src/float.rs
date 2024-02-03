use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use dashu_float::DBig;
use num_order::NumHash;
use pyo3::{exceptions::PyTypeError, intern, prelude::*, types::PyFloat};
type FBig = dashu_float::FBig;

use crate::{
    convert::{conversion_error_to_py, parse_error_to_py, parse_to_dbig},
    types::{DPy, FPy, IPy},
};

const ERRMSG_FBIG_WRONG_SRC_TYPE: &'static str =
    "only floats or strings can be used to construct an FBig instance";
const ERRMSG_DBIG_WRONG_SRC_TYPE: &'static str =
    "only Decimal instances or strings can be used to construct a DBig instance";

#[pymethods]
impl FPy {
    #[new]
    fn __new__(ob: &PyAny) -> PyResult<Self> {
        if ob.is_instance_of::<PyFloat>() {
            // create from float
            let f: f64 = ob.extract()?;
            let f = FBig::try_from(f).map_err(conversion_error_to_py)?;
            Ok(FPy(f))
        } else if let Ok(s) = ob.extract() {
            // create from string
            let f = FBig::from_str_native(s);
            Ok(FPy(f.map_err(parse_error_to_py)?))
        } else if let Ok(obj) = <PyRef<Self> as FromPyObject>::extract(ob) {
            Ok(FPy(obj.0.clone()))
        } else {
            Err(PyTypeError::new_err(ERRMSG_FBIG_WRONG_SRC_TYPE))
        }
    }
    fn unwrap(&self, py: Python) -> PyObject {
        let (signif, exp) = self.0.repr().clone().into_parts();
        (IPy(signif), exp).into_py(py)
    }

    fn __repr__(&self) -> String {
        format!("<FBig {:?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __format__(&self) {
        todo!()
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.num_hash(&mut hasher);
        hasher.finish()
    }

    fn __float__(&self) -> f64 {
        self.0.to_f64().value()
    }
}

#[pymethods]
impl DPy {
    #[new]
    fn __new__(ob: &PyAny) -> PyResult<Self> {
        let py = ob.py();
        let decimal = py.import(intern!(py, "decimal"))?;
        if ob.is_instance(decimal.getattr(intern!(py, "Decimal"))?)? {
            // create from decimal.Decimal
            Ok(DPy(parse_to_dbig(ob)?))
        } else if let Ok(s) = ob.extract() {
            // create from string
            let d = DBig::from_str_native(s);
            Ok(DPy(d.map_err(parse_error_to_py)?))
        } else if let Ok(obj) = <PyRef<Self> as FromPyObject>::extract(ob) {
            Ok(DPy(obj.0.clone()))
        } else {
            Err(PyTypeError::new_err(ERRMSG_DBIG_WRONG_SRC_TYPE))
        }
    }
    fn unwrap(&self, py: Python) -> PyResult<PyObject> {
        let decimal = py.import(intern!(py, "decimal"))?;
        let decimal_type = decimal.getattr(intern!(py, "Decimal"))?;
        let decimal_str = format!("{:e}", self.0);
        Ok(decimal_type.call1((decimal_str,))?.into())
    }

    fn __repr__(&self) -> String {
        format!("<DBig {:?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __format__(&self) {
        todo!()
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.num_hash(&mut hasher);
        hasher.finish()
    }

    fn __float__(&self) -> f64 {
        self.0.to_f64().value()
    }
}
