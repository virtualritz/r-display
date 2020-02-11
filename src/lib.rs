use ndspy_sys;

// Output TIFF for now.
#[macro_use]
extern crate tiff_encoder;
use tiff_encoder::ifd::tags;
use tiff_encoder::prelude::*;

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

use std::ffi::{c_void, CStr};
use std::str;

#[repr(C)]
#[derive(Debug)]
struct ImageData {
    data: Vec<u8>,
    offset: usize,
    width: u32,
    height: u32,
    file_name: String,
}

fn get_parameter(
    name: &str,
    parameter_count: std::os::raw::c_int,
    parameter: *const ndspy_sys::UserParameter,
) -> *const c_void {
    for i in 0..parameter_count {
        if name
            == unsafe {
                CStr::from_ptr((*parameter.offset(i as isize)).name)
                    .to_str()
                    .unwrap()
            }
        {
            return unsafe { (*parameter.offset(i as isize)).value };
        }
    }

    std::ptr::null()
}

#[no_mangle]
pub extern "C" fn DspyImageOpen(
    mut image_handle_ptr: *mut ndspy_sys::PtDspyImageHandle,
    _driver_name: *const std::os::raw::c_char,
    output_filename: *const std::os::raw::c_char,
    width: std::os::raw::c_int,
    height: std::os::raw::c_int,
    _parameter_count: std::os::raw::c_int,
    _parameter: *const ndspy_sys::UserParameter,
    _num_formats: std::os::raw::c_int,
    _formats: *const ndspy_sys::PtDspyDevFormat,
    flag_stuff: *mut ndspy_sys::PtFlagStuff,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageOpen()");

    println!("Handle received by the renderer: {:?}", image_handle_ptr,);

    println!("File name:                       {}", unsafe {
        CStr::from_ptr(output_filename).to_str().unwrap()
    });

    if (image_handle_ptr == std::ptr::null_mut()) || (output_filename == std::ptr::null_mut()) {
        return ndspy_sys::PtDspyError_PkDspyErrorBadParams;
    }

    let image = Box::new(ImageData {
        // We initialize the vector with zeros. While this could be
        // avoided using Vec::with_capacity() & Vec::set_len() this is
        // bad because it "exposes uninitialized memory to be read and
        // dropped on panic".
        // See https://github.com/rust-lang/rust-clippy/issues/4483
        data: vec![0; (width * height) as usize],
        offset: 0,
        width: width as u32,
        height: height as u32,
        file_name: unsafe {
            CStr::from_ptr(output_filename)
                .to_str()
                .unwrap()
                .to_string()
        },
    });

    //println!("Contents of ImageData struct:    {:?}", *image);

    // Get raw pointer to heap-allocated ImageData struct and pass
    // ownership to image_handle_ptr.
    image_handle_ptr = Box::into_raw(image) as *mut _;

    println!("Handle returned to renderer:     {:?}", image_handle_ptr);

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
    data_len: std::os::raw::c_int,
    mut data: *mut std::os::raw::c_void,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageQuery()");

    if (data == std::ptr::null_mut()) && (query_type != ndspy_sys::PtDspyQueryType_PkStopQuery) {
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
                if image_handle == std::ptr::null_mut() {
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

            assert!(std::mem::size_of::<ndspy_sys::PtDspySizeInfo>() <= data_len as usize);

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
    image_handle: *mut ndspy_sys::PtDspyImageHandle,
    x_min: std::os::raw::c_int,
    x_max_plus_one: std::os::raw::c_int,
    y_min: std::os::raw::c_int,
    y_max_plus_one: std::os::raw::c_int,
    entry_size: std::os::raw::c_int,
    data: *const std::os::raw::c_uchar,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageData()");

    let mut image = unsafe { Box::from_raw(image_handle as *mut ImageData) };

    if image_handle == std::ptr::null_mut() {
        return ndspy_sys::PtDspyError_PkDspyErrorBadParams;
    }

    let data_size = (entry_size * (x_max_plus_one - x_min) * (y_max_plus_one - y_min)) as usize;

    unsafe {
        std::ptr::copy_nonoverlapping(data, &mut image.data[image.offset], data_size);
    }

    image.offset += data_size;

    ndspy_sys::PtDspyError_PkDspyErrorNone
}

fn write_image(image: Box<ImageData>) -> Result<std::fs::File, std::io::Error> {
    TiffFile::new(
        Ifd::new()
            .with_entry(tags::PhotometricInterpretation, SHORT![1]) // Black is zero
            .with_entry(tags::Compression, SHORT![1]) // No compression
            .with_entry(tags::ImageWidth, LONG![image.width as u32])
            .with_entry(tags::ImageLength, LONG![image.height as u32])
            .with_entry(tags::ResolutionUnit, SHORT![1]) // No resolution unit
            .with_entry(tags::XResolution, RATIONAL![(1, 1)])
            .with_entry(tags::YResolution, RATIONAL![(1, 1)])
            .with_entry(tags::RowsPerStrip, LONG![image.width as u32]) // One strip for the whole image
            .with_entry(
                tags::StripByteCounts,
                LONG![(image.width * image.height) as u32],
            )
            .with_entry(tags::StripOffsets, ByteBlock::single(image.data))
            .single(), // This is the only Ifd in its IfdChain
    )
    .write_to(image.file_name)
}

#[no_mangle]
pub extern "C" fn DspyImageClose(
    image_handle: *mut ndspy_sys::PtDspyImageHandle,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageClose()");

    DspyImageDelayClose(image_handle)
}

#[no_mangle]
pub extern "C" fn DspyImageDelayClose(
    image_handle: *mut ndspy_sys::PtDspyImageHandle,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageDelayClose()");

    let image = unsafe { Box::from_raw(image_handle as *mut ImageData) };

    match write_image(image) {
        Ok(_file) => ndspy_sys::PtDspyError_PkDspyErrorNone,
        Err(e) => e
            .raw_os_error()
            .unwrap_or(ndspy_sys::PtDspyError_PkDspyErrorUndefined as i32) as u32,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(1 + 1, 2);
    }
}
