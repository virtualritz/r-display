#![allow(unused_assignments)]

use ndspy_sys;

// We output EXR instead once the Rust native EXR crate hits its first
// release. Building the (working) Rust C++ OpenEXR bindings involves
// a C++ toolchain (and dependencies) — too much pain in the ass™.
// Quote from the exrs crate: “Using the Rust bindings to OpenEXR
// requires compiling multiple C++ Libraries and setting environment
// variables, which I didn’t quite feel like to do, so I wrote this
// library instead.”
/*
use exr::prelude::*;
*/

use std::ffi::CStr;
use std::{fs, io, mem, os, path, ptr, slice, str};

use png;

#[repr(C)]
#[derive(Debug)]
struct ImageData {
    data: Vec<u8>,
    offset: isize,
    width: u32,
    height: u32,
    channels: u32,
    file_name: String,
}

/// A utility function to get user parameters.
///
/// The template argument is the expected type of the resp. parameter.
///
/// # Arguments
///
/// * `name` - A string slice that holds the name of the parameter we
///            are searching for
/// * `parameter_count` - Number of parameters
/// * `parameter`       - Array of `ndspy_sys::UserParameter` structs to
///                       search
///
/// # Example
///
/// ```
/// let associate_alpha =
///     1 == get_parameter::<i32>("associatealpha", _parameter_count, _parameter).unwrap_or(0);
/// ```
pub fn get_parameter<T: Copy>(
    name: &str,
    parameter_count: os::raw::c_int,
    parameter: *const ndspy_sys::UserParameter,
) -> Option<T> {
    for i in 0..parameter_count {
        if name
            == unsafe { CStr::from_ptr((*parameter.offset(i as isize)).name) }
                .to_str()
                .unwrap()
        {
            let value_ptr = (unsafe { *(parameter.offset(i as isize)) }).value as *const T;

            assert!(value_ptr != ptr::null());

            return Some(unsafe { *value_ptr });
        }
    }

    None
}

#[no_mangle]
pub extern "C" fn DspyImageOpen(
    image_handle_ptr: *mut ndspy_sys::PtDspyImageHandle,
    _driver_name: *const os::raw::c_char,
    output_filename: *const os::raw::c_char,
    width: os::raw::c_int,
    height: os::raw::c_int,
    parameter_count: os::raw::c_int,
    parameter: *const ndspy_sys::UserParameter,
    format_count: os::raw::c_int,
    format: *const ndspy_sys::PtDspyDevFormat,
    flag_stuff: *mut ndspy_sys::PtFlagStuff,
) -> ndspy_sys::PtDspyError {
    if (image_handle_ptr == ptr::null_mut()) || (output_filename == ptr::null_mut()) {
        return ndspy_sys::PtDspyError_PkDspyErrorBadParams;
    }

    // Example use of get_parameter() helper.
    let _associate_alpha =
        1 == get_parameter::<i32>("associatealpha", parameter_count, parameter).unwrap_or(0);

    // Ensure all channels are sent to us as 16bit integers.
    // This loops through each format (channel), r, g, b, a etc.
    for i in 0..format_count as isize {
        (unsafe { *format.offset(i) }).type_ = ndspy_sys::PkDspyUnsigned8;
    }

    let image = Box::new(ImageData {
        // We initialize the vector with zeros. While this could be
        // avoided using Vec::with_capacity() & Vec::set_len() this is
        // bad because it "exposes uninitialized memory to be read and
        // dropped on panic".
        // See https://github.com/rust-lang/rust-clippy/issues/4483
        data: vec![0; (width * height * format_count) as usize],
        offset: 0,
        width: width as u32,
        height: height as u32,
        channels: format_count as u32,
        file_name: unsafe {
            CStr::from_ptr(output_filename)
                .to_str()
                .unwrap()
                .to_string()
        },
    });

    // Get raw pointer to heap-allocated ImageData struct and pass
    // ownership to image_handle_ptr.
    unsafe {
        *image_handle_ptr = Box::into_raw(image) as *mut _;
    }

    // We're dereferencing a raw pointer – this is obviously unsafe
    unsafe {
        (*flag_stuff).flags |= ndspy_sys::PkDspyFlagsWantsScanLineOrder as i32;
    }

    ndspy_sys::PtDspyError_PkDspyErrorNone
}

#[no_mangle]
pub extern "C" fn DspyImageQuery(
    image_handle: ndspy_sys::PtDspyImageHandle,
    query_type: ndspy_sys::PtDspyQueryType,
    data_len: os::raw::c_int,
    mut data: *mut os::raw::c_void,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageQuery()");

    if (data == ptr::null_mut()) && (query_type != ndspy_sys::PtDspyQueryType_PkStopQuery) {
        return ndspy_sys::PtDspyError_PkDspyErrorBadParams;
    }

    // Looks like this is actually needed for a minimal implementation
    // as we never get called with the next two query types by 3Delight.
    // But we leave this code be – just in case. :]
    match query_type {
        ndspy_sys::PtDspyQueryType_PkSizeQuery => {
            println!("PkSizeQuery");
            let size_info = Box::new({
                println!("no size – using default");
                if image_handle == ptr::null_mut() {
                    ndspy_sys::PtDspySizeInfo {
                        width: 1920,
                        height: 1080,
                        aspectRatio: 1.0,
                    }
                } else {
                    println!("using size from exisiting image");

                    let image = unsafe { Box::from_raw(image_handle as *mut ImageData) };

                    ndspy_sys::PtDspySizeInfo {
                        width: image.width as u64,
                        height: image.height as u64,
                        aspectRatio: 1.0,
                    }
                }
            });

            assert!(mem::size_of::<ndspy_sys::PtDspySizeInfo>() <= data_len as usize);

            // Transfer ownership of the size_query heap object to the
            // data pointer.
            data = Box::into_raw(size_info) as *mut _;
        }

        ndspy_sys::PtDspyQueryType_PkOverwriteQuery => {
            println!("PkOverwriteQuery");

            let overwrite_info = Box::new(ndspy_sys::PtDspyOverwriteInfo {
                overwrite: true as ndspy_sys::PtDspyUnsigned8,
                unused: 0,
            });

            data = Box::into_raw(overwrite_info) as *mut _;
        }

        _ => {
            println!("Query: {:?}", query_type);
            return ndspy_sys::PtDspyError_PkDspyErrorUnsupported;
        }
    }

    ndspy_sys::PtDspyError_PkDspyErrorNone
}

#[no_mangle]
pub extern "C" fn DspyImageData(
    image_handle: ndspy_sys::PtDspyImageHandle,
    x_min: os::raw::c_int,
    x_max_plus_one: os::raw::c_int,
    y_min: os::raw::c_int,
    y_max_plus_one: os::raw::c_int,
    _entry_size: os::raw::c_int,
    data: *const os::raw::c_uchar,
) -> ndspy_sys::PtDspyError {
    let mut image = unsafe { Box::from_raw(image_handle as *mut ImageData) };

    if image_handle == ptr::null_mut() {
        return ndspy_sys::PtDspyError_PkDspyErrorBadParams;
    }

    let data_size =
        (image.channels as i32 * (x_max_plus_one - x_min) * (y_max_plus_one - y_min)) as usize;

    unsafe {
        ptr::copy_nonoverlapping(
            data,
            image.data.as_mut_ptr().offset(image.offset),
            data_size,
        );
    }

    image.offset += data_size as isize;

    // Important: we need to give up ownership of the boxed image or
    // else the compiler will free the memory on exiting this function.
    Box::into_raw(image);

    ndspy_sys::PtDspyError_PkDspyErrorNone
}

fn write_image(image: Box<ImageData>) -> Result<(), png::EncodingError> {
    let path = path::Path::new(&image.file_name);
    let file = fs::File::create(path).unwrap();
    let ref mut writer = io::BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, image.width, image.height);
    encoder.set_color(png::ColorType::RGBA);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    writer.write_image_data(unsafe {
        slice::from_raw_parts(image.data.as_ptr() as *const u8, image.data.len())
    })
}

#[no_mangle]
pub extern "C" fn DspyImageClose(
    image_handle: ndspy_sys::PtDspyImageHandle,
) -> ndspy_sys::PtDspyError {
    DspyImageDelayClose(image_handle)
}

#[no_mangle]
pub extern "C" fn DspyImageDelayClose(
    image_handle: ndspy_sys::PtDspyImageHandle,
) -> ndspy_sys::PtDspyError {
    let image = unsafe { Box::from_raw(image_handle as *mut ImageData) };

    match write_image(image) {
        Ok(_) => ndspy_sys::PtDspyError_PkDspyErrorNone,
        Err(_) => ndspy_sys::PtDspyError_PkDspyErrorUndefined,
    }

    // image goes out of scope – this will free the memory
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(1 + 1, 2);
    }
}
