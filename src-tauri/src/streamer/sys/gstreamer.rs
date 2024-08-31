use std::{
    collections::VecDeque,
    ffi::{c_char, CString},
    ptr::{null, null_mut, NonNull},
};

use gstreamer_sys::{gst_init, gst_parse_launch, gst_structure_new, GstElement, GstStructure};

use super::{
    element::Element,
    structure::{self, Structure},
    structure_field,
};

pub(crate) fn init() {
    let args = std::env::args()
        .map(|arg| CString::new(arg).unwrap())
        .collect::<Vec<CString>>();

    let mut c_args = args
        .iter()
        .map(|arg| arg.clone().into_raw())
        .collect::<Vec<*mut c_char>>();

    unsafe { gst_init(&mut (c_args.len() as i32), &mut c_args.as_mut_ptr()) };
}

pub(crate) fn parse_launch(uri: &str) -> Result<Element, String> {
    let pipeline_description = CString::new(format!("playbin uri=\"{uri}\""))
        .or_else(|_| Err(format!("Error on pipeline description conversion.")))?;

    let element_ptr = unsafe { gst_parse_launch(pipeline_description.as_ptr(), null_mut()) };

    if let Some(element) = NonNull::new(element_ptr as *mut GstElement) {
        return Ok(Element::new(element));
    }

    Err(format!("The pipeline is null."))
}

pub(crate) fn structure_new(
    name: &str,
    fields: Vec<Box<dyn structure_field::Field>>,
) -> Result<Structure, String> {
    let c_name = CString::new(name).unwrap();
    let mut c_fields = Vec::<structure::CField>::new();
    let mut c_values = VecDeque::<*const c_char>::new();

    for field in fields {
        let field_name = CString::new(field.field_name()).unwrap();
        let g_type = field.g_type();
        let value = field.c_value();
        c_values.push_back(field_name.as_ptr());
        c_values.push_back(g_type as *const c_char);
        c_values.push_back(std::ptr::addr_of!(*value) as *const c_char);
        c_fields.push((field_name, g_type, value));
    }

    c_values.push_back(null());
    let first_field = c_values.pop_front().unwrap();
    let structure_ptr = unsafe { gst_structure_new(c_name.as_ptr(), first_field, c_values) };

    if let Some(structure) = NonNull::new(structure_ptr as *mut GstStructure) {
        return Ok(Structure::new(structure, Some((c_name, c_fields))));
    }

    Err("GStreamer returned a null structure for the message.".to_owned())
}
