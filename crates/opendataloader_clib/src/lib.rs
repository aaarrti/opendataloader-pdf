use std::ffi::{CStr, c_uint};
use std::os::raw::{c_char, c_int};

/// int odl_convert(char** pdf_paths, unsigned int n_paths, char* output_dir, unsigned int output_mode)
/// different bits encode json on/off markdown on/off, images on/off
#[unsafe(no_mangle)]
pub extern "C" fn odl_convert(
    pdf_paths: *const c_char,
    n_paths: c_uint,
    out_dir: *const c_char,
    mode: c_uint,
) -> c_int {
    0 as c_int
}
