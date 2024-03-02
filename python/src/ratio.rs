use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use dashu_ratio::RBig;
use num_order::NumHash;
use pyo3::{exceptions::PyTypeError, intern, prelude::*};

use crate::{
    convert::{convert_from_rbig, parse_error_to_py, parse_to_rbig},
    types::RPy,
};

const ERRMSG_RBIG_WRONG_SRC_TYPE: &str =
    "only Fraction instances or strings can be used to construct an RBig instance";

#[pymethods]
impl RPy {
    #[new]
    fn __new__(ob: &PyAny) -> PyResult<Self> {
        let py = ob.py();
        let fractions = py.import(intern!(py, "fractions"))?;
        if ob.is_instance(fractions.getattr(intern!(py, "Fraction"))?)? {
            // create from fractions.Fraction
            Ok(RPy(parse_to_rbig(ob)?))
        } else if let Ok(s) = ob.extract() {
            // create from string
            let d = RBig::from_str_with_radix_prefix(s).map(|v| v.0);
            Ok(RPy(d.map_err(parse_error_to_py)?))
        } else if let Ok(obj) = <PyRef<Self> as FromPyObject>::extract(ob) {
            Ok(RPy(obj.0.clone()))
        } else {
            Err(PyTypeError::new_err(ERRMSG_RBIG_WRONG_SRC_TYPE))
        }
    }
    fn unwrap(&self, py: Python) -> PyResult<PyObject> {
        convert_from_rbig(&self.0, py)
    }

    fn __repr__(&self) -> String {
        format!("<RBig {:?}>", self.0)
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
        self.0.to_f64_fast()
    }
}
