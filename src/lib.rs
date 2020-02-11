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
use std::{fs, io, mem, os, ptr, str};

#[repr(C)]
#[derive(Debug)]
struct ImageData {
    data: Vec<u8>,
    offset: isize,
    width: u32,
    height: u32,
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
    _parameter_count: os::raw::c_int,
    _parameter: *const ndspy_sys::UserParameter,
    format_count: os::raw::c_int,
    format: *const ndspy_sys::PtDspyDevFormat,
    flag_stuff: *mut ndspy_sys::PtFlagStuff,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageOpen()");

    println!("Handle received from the renderer: {:?}", image_handle_ptr,);

    println!("File name:                         {}", unsafe {
        CStr::from_ptr(output_filename).to_str().unwrap()
    });

    if (image_handle_ptr == ptr::null_mut()) || (output_filename == ptr::null_mut()) {
        return ndspy_sys::PtDspyError_PkDspyErrorBadParams;
    }

    let associate_alpha =
        1 == get_parameter::<i32>("associatealpha", _parameter_count, _parameter).unwrap_or(0);

    let mut active_format = (unsafe { *format.offset(0) }).type_ & ndspy_sys::PkDspyMaskType;
    let mut bits: u8 = 8;
    let mut white_value: f32 = 255.0;
    let mut is_float = false;

    match active_format {
        ndspy_sys::PkDspyFloat16 => {
            bits = 16;
            white_value = 1.0;
            is_float = true;
        }
        ndspy_sys::PkDspyFloat32 => {
            bits = 32;
            white_value = 1.0;
            is_float = true;
        }
        ndspy_sys::PkDspyUnsigned16 => {
            bits = 16;
            white_value = 65535.0;
        }
        _ => {
            active_format = ndspy_sys::PkDspyUnsigned8;
        }
    }

    // Ensure all channels have the same format.
    for i in 0..format_count {
        (unsafe { *format.offset(0) }).type_ = active_format;
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

    //println!("Contents of ImageData struct:      {:?}", *image);

    // Get raw pointer to heap-allocated ImageData struct and pass
    // ownership to image_handle_ptr.
    unsafe {
        *image_handle_ptr = Box::into_raw(image) as *mut _;
    }

    println!("Handle returned to renderer:       {:?}", unsafe {
        *image_handle_ptr
    });

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
    entry_size: os::raw::c_int,
    data: *const os::raw::c_uchar,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageData()");

    let mut image = unsafe { Box::from_raw(image_handle as *mut ImageData) };

    println!("Handle received from the renderer: {:?}", image_handle);

    if image_handle == ptr::null_mut() {
        return ndspy_sys::PtDspyError_PkDspyErrorBadParams;
    }

    println!("Contents of ImageData struct:      {:?}", image);

    let data_size = (entry_size * (x_max_plus_one - x_min) * (y_max_plus_one - y_min)) as usize;

    println!("Data size to copy:                 {}", data_size);
    println!("Offset:                            {}", image.offset);
    println!("Entry size:                        {}", entry_size);

    unsafe {
        ptr::copy_nonoverlapping(
            data,
            image.data.as_mut_ptr().offset(image.offset),
            data_size,
        );
    }

    image.offset += data_size as isize;

    ndspy_sys::PtDspyError_PkDspyErrorNone
}

fn write_image(image: Box<ImageData>) -> Result<fs::File, io::Error> {
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
    image_handle: ndspy_sys::PtDspyImageHandle,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageClose()");

    DspyImageDelayClose(image_handle)
}

#[no_mangle]
pub extern "C" fn DspyImageDelayClose(
    image_handle: ndspy_sys::PtDspyImageHandle,
) -> ndspy_sys::PtDspyError {
    println!("DspyImageDelayClose()");

    let image = unsafe { Box::from_raw(image_handle as *mut ImageData) };

    match write_image(image) {
        Ok(_file) => ndspy_sys::PtDspyError_PkDspyErrorNone,
        Err(e) => e
            .raw_os_error()
            .unwrap_or(ndspy_sys::PtDspyError_PkDspyErrorUndefined as i32) as u32,
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
