#![allow(unused_assignments)]
extern crate exr;
use c_vec::CVec;
use exr::prelude::rgba_image::*;
use ndspy_sys;
//use oidn;
use cgmath::prelude::*;
use std::{
    ffi::CStr,
    mem,
    os::raw::{c_char, c_int, c_void},
    ptr,
};

//use png;

#[repr(C)]
#[derive(Debug)]
struct ImageData {
    data: Vec<f32>,
    offset: isize,
    width: usize,
    height: usize,
    pixel_aspect: f32,
    world_to_screen: Option<[f32; 16]>,
    world_to_camera: Option<[f32; 16]>,
    near: Option<f32>,
    far: Option<f32>,
    /*renderer: Option<Text>,
    data_window: [f32; 4],
    display_window: [f32; 4],
    fov_vertical: f32,
    fov_horizontal: f32,
    screen_window_center: [f32; 2],
    screen_window_width: f32,*/
    channels: usize,
    premultiply: bool,
    compression: Compression,
    line_order: Option<LineOrder>,
    tile_size: Option<Vec2<usize>>,
    file_name: String,
    denoise: bool,
}

impl ImageData {
    fn unpremultiply(&mut self) {
        for i in (0..self.data.len()).step_by(4) {
            let alpha = self.data[i + 3];
            if alpha != 0.0f32 {
                for c in i..i + 3 {
                    self.data[c] /= alpha;
                }
            }
        }
    }

    fn premultiply(&mut self) {
        for i in (0..self.data.len()).step_by(4) {
            let alpha = self.data[i + 3];
            if alpha != 0.0f32 {
                for c in i..i + 3 {
                    self.data[c] *= alpha;
                }
            }
        }
    }
}

type Type = u8;

/// A utility function to get user parameters.
///
/// The template argument is the expected type of the resp. parameter.
///
/// # Arguments
///
/// * `name` - A string slice that holds the name of the parameter we
///   are searching for
/// * `parameter_count` - Number of parameters
/// * `parameter`       - Array of `ndspy_sys::UserParameter` structs to
///   search
///
/// # Example
///
/// ```
/// let associate_alpha =
///     1 == get_parameter::<i32>("associatealpha", _parameter_count, _parameter).unwrap_or(0);
/// ```
pub fn get_parameter<T: Copy>(
    name: &str,
    type_: Type,
    len: usize,
    parameter: &c_vec::CVec<ndspy_sys::UserParameter>,
) -> Option<T> {
    for p in parameter.iter() {
        let p_name = unsafe { CStr::from_ptr(p.name) }.to_str().unwrap();

        if name == p_name && type_ == p.valueType as u8 && len == p.valueCount as usize {
            let value_ptr = p.value as *const T;

            if value_ptr != ptr::null() {
                return Some(unsafe { *value_ptr });
            } else {
                // Value is missing, exit quietly.
                break;
            }
        }
    }

    None
}

#[no_mangle]
pub extern "C" fn DspyImageOpen(
    image_handle_ptr: *mut ndspy_sys::PtDspyImageHandle,
    _driver_name: *const c_char,
    output_filename: *const c_char,
    width: c_int,
    height: c_int,
    parameter_count: c_int,
    parameter: *mut ndspy_sys::UserParameter,
    format_count: c_int,
    format: *mut ndspy_sys::PtDspyDevFormat,
    flag_stuff: *mut ndspy_sys::PtFlagStuff,
) -> ndspy_sys::PtDspyError {
    if (image_handle_ptr == ptr::null_mut()) || (output_filename == ptr::null_mut()) {
        return ndspy_sys::PtDspyError_PkDspyErrorBadParams;
    }

    // Shadow C
    let mut format = // : Vec<ndspy_sys::PtDspyDevFormat> =
        unsafe { CVec::new(format, format_count as usize) }; //.into();

    // Ensure all channels are sent to us as 8bit integers.
    // This loops through each format (channel), r, g, b, a etc.
    // We also dump all formats to stderr.
    for i in 0..format.len() {
        format.get_mut(i).unwrap().type_ = ndspy_sys::PkDspyFloat32;
        /*eprintln!("{:?}", unsafe {
            CStr::from_ptr(format.get(i).unwrap().name)
        });*/
    }

    // Shadow C paramater array with wrapped version
    let parameter = unsafe { CVec::new(parameter, parameter_count as usize) };

    if output_filename != std::ptr::null() {
        let image = Box::new(ImageData {
            data: vec![0.0f32; (width * height * format_count) as usize],
            offset: 0,

            width: width as usize,
            height: height as usize,
            pixel_aspect: get_parameter::<f32>("PixelAspectRatio", b'f', 1, &parameter)
                .unwrap_or(1.0f32),

            world_to_screen: get_parameter::<[f32; 16]>("NP", b'f', 16, &parameter),
            world_to_camera: get_parameter::<[f32; 16]>("Nl", b'f', 16, &parameter),

            near: get_parameter::<f32>("near", b'f', 1, &parameter),
            far: get_parameter::<f32>("far", b'f', 1, &parameter),
            /*renderer: match get_parameter::<*const std::os::raw::c_char>("Software", &parameter) {
                Some(c_str_ptr) => Some(
                    unsafe { CStr::from_ptr(c_str_ptr) }
                        .to_string_lossy()
                        .into_owned()
                        .as_str()
                        .try_into()
                        .unwrap(),
                ),
                None => None,
            },
            fov_vertical: f32,
            fov_horizontal: f32,
            screen_window_center: [f32; 2],
            screen_window_width: f32,
                */
            channels: format_count as usize,

            premultiply: match get_parameter::<u32>("premultiply", b'i', 1, &parameter) {
                Some(b) => b != 0,
                None => true,
            },

            compression: match get_parameter::<*const std::os::raw::c_char>(
                "compression",
                b's',
                1,
                &parameter,
            ) {
                None => Compression::ZIP16,
                Some(c_str_ptr) => match unsafe { CStr::from_ptr(c_str_ptr) }
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "none" => Compression::Uncompressed,
                    "rle" => Compression::RLE,
                    "piz" => Compression::PIZ,
                    "pxr24" => Compression::PXR24,
                    "zip" => Compression::ZIP16,
                    _ => {
                        eprintln!("[r-display] selected compression is not supported; reverting to 'zip'");
                        Compression::ZIP16
                    }
                },
            },

            line_order: match get_parameter::<*const std::os::raw::c_char>(
                "line_order",
                b's',
                1,
                &parameter,
            ) {
                None => None,
                Some(c_str_ptr) => match unsafe { CStr::from_ptr(c_str_ptr) }
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "increasing" => Some(LineOrder::Increasing),
                    "decreasing" => Some(LineOrder::Decreasing),
                    _ => {
                        eprintln!("[r-display] selected line_order is not supported; ignoring");
                        None
                    }
                },
            },

            tile_size: match get_parameter::<[u32; 2]>("tile_size", b'i', 2, &parameter) {
                None => None,
                Some(t) => Some(Vec2::from((t[0] as usize, t[1] as usize))),
            },

            file_name: unsafe {
                CStr::from_ptr(output_filename)
                    .to_str()
                    .unwrap()
                    .to_string()
            },

            denoise: match get_parameter::<u32>("denoise", b'i', 1, &parameter) {
                Some(b) => b != 0,
                None => true,
            },
        });

        eprintln!("{:?}", image.denoise);

        // Get raw pointer to heap-allocated ImageData struct and pass
        // ownership to image_handle_ptr.
        unsafe {
            *image_handle_ptr = Box::into_raw(image) as *mut _;
        }

        unsafe {
            (*flag_stuff).flags |= ndspy_sys::PkDspyFlagsWantsScanLineOrder as i32;
        }

        ndspy_sys::PtDspyError_PkDspyErrorNone
    } else {
        // We're missing an output file name.
        ndspy_sys::PtDspyError_PkDspyErrorBadParams
    }
}

#[no_mangle]
pub extern "C" fn DspyImageQuery(
    image_handle: ndspy_sys::PtDspyImageHandle,
    query_type: ndspy_sys::PtDspyQueryType,
    data_len: c_int,
    mut data: *const c_void,
) -> ndspy_sys::PtDspyError {
    if (data == ptr::null_mut()) && (query_type != ndspy_sys::PtDspyQueryType_PkStopQuery) {
        return ndspy_sys::PtDspyError_PkDspyErrorBadParams;
    }

    // Looks like this is actually needed for a minimal implementation
    // as we never get called with the next two query types by 3Delight.
    // But we leave this code be – just in case. :]
    match query_type {
        ndspy_sys::PtDspyQueryType_PkSizeQuery => {
            let size_info = Box::new({
                if image_handle == ptr::null_mut() {
                    ndspy_sys::PtDspySizeInfo {
                        width: 1920,
                        height: 1080,
                        aspectRatio: 1.0,
                    }
                } else {
                    let image = unsafe { Box::from_raw(image_handle as *mut ImageData) };

                    ndspy_sys::PtDspySizeInfo {
                        width: image.width as u64,
                        height: image.height as u64,
                        aspectRatio: image.pixel_aspect,
                    }
                }
            });

            debug_assert!(mem::size_of::<ndspy_sys::PtDspySizeInfo>() <= data_len as usize);

            // Transfer ownership of the size_query heap object to the
            // data pointer.
            data = Box::into_raw(size_info) as *mut _;
        }

        ndspy_sys::PtDspyQueryType_PkOverwriteQuery => {
            let overwrite_info = Box::new(ndspy_sys::PtDspyOverwriteInfo {
                overwrite: true as ndspy_sys::PtDspyUnsigned8,
                unused: 0,
            });

            // Transfer ownership of the size_query heap object to the
            // data pointer.
            data = Box::into_raw(overwrite_info) as *mut _;
        }

        _ => {
            return ndspy_sys::PtDspyError_PkDspyErrorUnsupported;
        }
    }

    ndspy_sys::PtDspyError_PkDspyErrorNone
}

#[no_mangle]
pub extern "C" fn DspyImageData(
    image_handle: ndspy_sys::PtDspyImageHandle,
    x_min: c_int,
    x_max_plus_one: c_int,
    y_min: c_int,
    y_max_plus_one: c_int,
    _entry_size: c_int,
    data: *const f32,
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

fn add_field_of_views(layer_attributes: &mut LayerAttributes) {
    if layer_attributes.world_to_camera == None
        || layer_attributes.world_to_normalized_device == None
    {
        return;
    }

    let w_to_cam_tmp = layer_attributes.world_to_camera.unwrap();
    let w_to_ndc_tmp = layer_attributes.world_to_normalized_device.unwrap();
    let w_to_cam: &cgmath::Matrix4<f32> = (&w_to_cam_tmp).into();
    let w_to_ndc: &cgmath::Matrix4<f32> = (&w_to_ndc_tmp).into();

    if w_to_ndc[2][3] == 0. {
        return;
    }

    let w_to_ndc_inv = w_to_ndc.invert();

    if None == w_to_ndc_inv {
        return;
    }

    let m = w_to_ndc_inv.unwrap() * *w_to_cam;
    let _v = m * cgmath::Vector4::<f32>::new(1., 1., 0., 0.);

    /*layer_attributes.horizontal_field_of_view =
        v.x.atan() * 360. / std::f32::consts::PI;
    layer_attributes.vertical_field_of_view =
        v.y.atan() * 360. / std::f32::consts::PI;*/
}

fn write_exr(image: &Box<ImageData>) {
    // -> Result<(), std::boxed::Box<dyn std::error::Error>> {
    let sample = |position: Vec2<usize>| {
        let index = image.channels * (position.x() + position.y() * image.width);

        Pixel::rgba(
            image.data[index],
            image.data[index + 1],
            image.data[index + 2],
            image.data[index + 3],
        )
    };

    let mut image_info = ImageInfo::rgba((image.width, image.height), SampleType::F32);

    image_info.image_attributes.pixel_aspect = image.pixel_aspect;

    //image_info.layer_attributes.comments = image.renderer;

    image_info.layer_attributes.world_to_camera = image.world_to_camera;
    image_info.layer_attributes.world_to_normalized_device = image.world_to_screen;

    add_field_of_views(&mut image_info.layer_attributes);

    let mut encoding = Encoding::for_compression(image.compression);

    if let Some(l) = image.line_order {
        encoding.line_order = l;
    }

    if let Some(s) = image.tile_size {
        encoding.tile_size = Some(s);
    }

    // write it to a file with all cores in parallel
    image_info
        .with_encoding(encoding)
        .write_pixels_to_file(
            image.file_name.clone(),
            // this will actually generate the pixels in parallel on all cores
            write_options::high(),
            &sample,
        )
        .unwrap();

    //    Ok(())ls
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
    let image = unsafe { &mut Box::from_raw(image_handle as *mut ImageData) };

    if image.denoise {
        let device = oidn::Device::new();
        let mut filter = oidn::RayTracing::new(&device);
        // Optionally add float3 normal and albedo buffers as well
        filter
            .set_img_dims(image.width as usize, image.height as usize)
            // .set_normal()
            // .set_albedo()
            .set_hdr(true);

        image.unpremultiply();

        let rgb: Vec<f32> = image
            .data
            .chunks(4)
            .flat_map(|c| c.iter().copied().take(3))
            .collect();

        let mut denoised_rgb = vec![0.0f32; rgb.len()];

        filter
            .execute(&rgb[..], &mut denoised_rgb[..])
            .expect("[r-display] Error denoising image.");

        let mut image_data_rgba_iter = image.data.chunks_mut(4);
        denoised_rgb
            .chunks(3)
            .for_each(|c| image_data_rgba_iter.next().unwrap()[0..3].copy_from_slice(c));

        if image.premultiply {
            image.premultiply();
        }
    } else if !image.premultiply {
        image.unpremultiply();
    }

    write_exr(&image);
    /*
    match write_png(&image) {
        Ok(_) => ndspy_sys::PtDspyError_PkDspyErrorNone,
        Err(_) => ndspy_sys::PtDspyError_PkDspyErrorUndefined,
    }*/

    // image goes out of scope – this will free the memory
    ndspy_sys::PtDspyError_PkDspyErrorNone
}
