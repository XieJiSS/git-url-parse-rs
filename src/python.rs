use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::exceptions::PyValueError;
use crate::GitUrl;

#[pyfunction]
fn parse(url: &str) -> PyResult<Py<PyDict>> {
    Python::with_gil(|py| {
        match GitUrl::parse(url) {
            Ok(git_url) => {
                let dict = PyDict::new(py);
                
                dict.set_item("host", git_url.host)?;
                dict.set_item("name", git_url.name)?;
                dict.set_item("owner", git_url.owner)?;
                dict.set_item("subgroups", git_url.subgroups)?;
                dict.set_item("organization", git_url.organization)?;
                dict.set_item("fullname", git_url.fullname)?;
                dict.set_item("scheme", git_url.scheme.to_string())?;
                dict.set_item("auth_user", git_url.auth_user)?;
                dict.set_item("auth_token", git_url.auth_token)?;
                dict.set_item("port", git_url.port)?;
                dict.set_item("path", git_url.path)?;
                dict.set_item("git_suffix", git_url.git_suffix)?;
                dict.set_item("scheme_prefix", git_url.scheme_prefix)?;
                
                Ok(dict.into())
            }
            Err(e) => Err(PyValueError::new_err(e.to_string())),
        }
    })
}

#[pymodule]
fn git_url_parse(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}
