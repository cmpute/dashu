use crate::types::PyWords;

use pyo3::{
    exceptions::{PyIndexError, PyTypeError},
    prelude::*,
    types::PySlice,
};

use dashu_int::Word;

#[pymethods]
impl PyWords {
    fn __repr__(&self) -> String {
        format!("<Words with {} items>", self.0.len())
    }
    fn __str__(&self) -> String {
        format!("{:?}", self.0)
    }
    fn __len__(&self) -> usize {
        self.0.len()
    }
    fn __getitem__(&self, index: &PyAny) -> PyResult<Word> {
        if let Ok(n) = <usize as FromPyObject>::extract(index) {
            self.0
                .get(n)
                .copied()
                .ok_or(PyIndexError::new_err("words index out of range"))
        } else if let Ok(slice) = index.downcast::<PySlice>() {
            todo!()
        } else {
            Err(PyTypeError::new_err("words indices must be integers or slices"))
        }
    }
    fn __setitem__(&mut self, index: &PyAny, value: Word) {
        todo!()
    }
}
