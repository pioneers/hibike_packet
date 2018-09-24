use ::cpython::{PyErr, Python, PyObject, PythonObject, ToPyObject, exc};

/// Create a `ValueError` with a message `msg`.
pub fn value_error(gil: Python, msg: String) -> PyErr {
    PyErr::new::<exc::ValueError, _>(gil, msg) 
}

/// Turn something into a Python `object`.
pub fn objectify<T>(gil: Python, obj: T) -> PyObject where T: ToPyObject {
    obj.into_py_object(gil).into_object()
}

/// Assert that a condition holds. Raises `AssertionError` should the condition fail to hold.
macro_rules! py_assert {
    ($py: ident, $cond: expr) => (
        if !$cond {
            return Err(PyErr::new::<exc::AssertionError, _>($py, concat!("Assertion failed: ", stringify!(cond))));
        }
    );

    ($py: ident, $cond: expr, $msg: expr) => (
        if !$cond {
            return Err(PyErr::new::<exc::AssertionError, _>($py, format!("Assertion failed: {}", $msg)));
        }
    );
}

