use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use std::ffi::c_void;

use calyx::ir;

unsafe impl Send for Component {}

#[pyclass]
#[derive(Clone)]
struct Component {
    comp: *mut c_void,
}

fn raw_transform(comp: Box<ir::Component>) -> Component {
    Component {
        comp: Box::into_raw(comp) as *mut c_void,
    }
}

#[pyfunction]
fn new_component(
    name: &str,
    inputs: Vec<(&str, u64)>,
    outputs: Vec<(&str, u64)>,
) -> Component {
    let comp = Box::new(ir::Component::new(name, inputs, outputs));
    Component {
        comp: Box::into_raw(comp) as *mut c_void,
    }
}

#[pyfunction]
fn get_component_name(c: Component) -> String {
    unsafe {
        let comp_raw = c.comp as *mut ir::Component;
        let comp = Box::from_raw(comp_raw);
        let ret = comp.name.to_string();
        raw_transform(comp);
        ret
    }
}

#[pyfunction]
fn free_component(c: Component) -> () {
    unsafe {
        let comp_raw = c.comp as *mut ir::Component;
        Box::from_raw(comp_raw);
    }
}

#[pymodule]
fn libcalyx_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(new_component, m)?)?;
    m.add_function(wrap_pyfunction!(get_component_name, m)?)?;
    m.add_function(wrap_pyfunction!(free_component, m)?)?;

    Ok(())
}
