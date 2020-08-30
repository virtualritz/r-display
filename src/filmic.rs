use core::arch::x86_64::*;
use rayon::prelude::*;

struct Filmic {
  grey_point_source: f32,
  black_point_source: f32,
  white_point_source: f32,
  security_factor: f32,
  grey_point_target: f32,
  black_point_target: f32,
  white_point_target: f32,
  output_gamma: f32,
  latitude_stops: f32,
  contrast: f32,
  saturation: f32,
  balance: f32,
  preserve_color: bool,
}

/// Uses D50 white point.
/// See http://www.brucelindbloom.com/Eqn_RGB_XYZ_Matrix.html for the
/// transformation matrices.
unsafe fn RGB_to_XYZ_sse2(rgb: __m128) -> __m128 {
  // sRGB -> XYZ matrix, D65
  let srgb_to_xyz_0: __m128 =
    _mm_setr_ps(0.4360747f32, 0.2225045, 0.0139322, 0.0);
  let srgb_to_xyz_1: __m128 =
    _mm_setr_ps(0.3850649f32, 0.7168786, 0.0971045, 0.0);
  let srgb_to_xyz_2: __m128 =
    _mm_setr_ps(0.1430804f32, 0.0606169, 0.7141733, 0.0);

  _mm_add_ps(
    _mm_mul_ps(
      srgb_to_xyz_0,
      _mm_shuffle_ps(rgb, rgb, _MM_SHUFFLE(0, 0, 0, 0)),
    ),
    _mm_add_ps(
      _mm_mul_ps(
        srgb_to_xyz_1,
        _mm_shuffle_ps(rgb, rgb, _MM_SHUFFLE(1, 1, 1, 1)),
      ),
      _mm_mul_ps(
        srgb_to_xyz_2,
        _mm_shuffle_ps(rgb, rgb, _MM_SHUFFLE(2, 2, 2, 2)),
      ),
    ),
  )
}

fn filmic_spline(
  x: f32,
  m1: [f32; 4],
  m2: [f32; 4],
  m3: [f32; 4],
  m4: [f32; 4],
  m5: [f32; 4],
  latitude_min: f32,
  latitude_max: f32,
) -> f32 {
  let toe = if x < latitude_min { 1.0f32 } else { 0.0 };
  let shoulder = if x > latitude_max { 1.0f32 } else { 0.0 };
  let latitude = if toe == shoulder { 1.0f32 } else { 0.0 }; // == FALSE
  let mask = [toe, shoulder, latitude];

  // Parallel loop
  (0usize..3)
    .into_par_iter()
    .map(|i| {
      mask[i]
        * (m1[i] + x * (m2[i] + x * (m3[i] + x * (m4[i] + x * m5[i]))))
    })
    .sum()
}


impl Filmic
{
    pub fn apply(data: Vec<f32>)
    {

        const dt_iop_filmicrgb_data_t *const data = (dt_iop_filmicrgb_data_t *)piece->data;
        const dt_iop_order_iccprofile_info_t *const work_profile = dt_ioppr_get_pipe_work_profile_info(piece->pipe);

        const int ch = piece->colors;

        /** The log2(x) -> -INF when x -> 0
        * thus very low values (noise) will get even lower, resulting in noise negative amplification,
        * which leads to pepper noise in shadows. To avoid that, we need to clip values that are noise for sure.
        * Using 16 bits RAW data, the black value (known by rawspeed for every manufacturer) could be used as a threshold.
        * However, at this point of the pixelpipe, the RAW levels have already been corrected and everything can happen with black levels
        * in the exposure module. So we define the threshold as the first non-null 16 bit integer
        */
       

        const float *const restrict in = (float *)ivoid;
        float *const restrict out = (float *)ovoid;

        const int variant = data->preserve_color;
        const dt_iop_filmic_rgb_spline_t spline = (dt_iop_filmic_rgb_spline_t)data->spline;

        if(variant == DT_FILMIC_METHOD_NONE) // no chroma preservation
  {
#ifdef _OPENMP
#pragma omp parallel for simd default(none) \
  dt_omp_firstprivate(ch, data, in, out, roi_out, work_profile, spline) \
  schedule(simd:static) aligned(in, out:64)
#endif
    for(size_t k = 0; k < roi_out->height * roi_out->width * ch; k += ch)
    {
      const float *const pix_in = in + k;
      float *const pix_out = out + k;
      float DT_ALIGNED_PIXEL temp[4];

      // Log tone-mapping
      for(int c = 0; c < 3; c++)
        temp[c] = log_tonemapping((pix_in[c] < 1.52587890625e-05f) ? 1.52587890625e-05f : pix_in[c],
                                   data->grey_source, data->black_source, data->dynamic_range);

      // Get the desaturation coeff based on the log value
      const float lum = (work_profile) ? dt_ioppr_get_rgb_matrix_luminance(temp,
                                                                           work_profile->matrix_in,
                                                                           work_profile->lut_in,
                                                                           work_profile->unbounded_coeffs_in,
                                                                           work_profile->lutsize,
                                                                           work_profile->nonlinearlut)
                                        : dt_camera_rgb_luminance(temp);
      const float desaturation = filmic_desaturate(lum, data->sigma_toe, data->sigma_shoulder, data->saturation);

      // Desaturate on the non-linear parts of the curve
      // Filmic S curve on the max RGB
      // Apply the transfer function of the display
      for(int c = 0; c < 3; c++)
        pix_out[c] = powf(clamp_simd(filmic_spline(linear_saturation(temp[c], lum, desaturation), spline.M1, spline.M2, spline.M3, spline.M4, spline.M5, spline.latitude_min, spline.latitude_max)), data->output_power);

    }
  }
  else // chroma preservation
  {
#ifdef _OPENMP
#pragma omp parallel for simd default(none) \
  dt_omp_firstprivate(ch, data, in, out, roi_out, work_profile, variant, spline) \
  schedule(simd:static) aligned(in, out:64)
#endif
    for(size_t k = 0; k < roi_out->height * roi_out->width * ch; k += ch)
    {
      const float *const pix_in = in + k;
      float *const pix_out = out + k;

      float DT_ALIGNED_PIXEL ratios[4];
      float norm = get_pixel_norm(pix_in, variant, work_profile);

      norm = (norm < 1.52587890625e-05f) ? 1.52587890625e-05f : norm; // norm can't be < to 2^(-16)

      // Save the ratios
      for(int c = 0; c < 3; c++) ratios[c] = pix_in[c] / norm;

      // Sanitize the ratios
      const float min_ratios = fminf(fminf(ratios[0], ratios[1]), ratios[2]);
      if(min_ratios < 0.0f) for(int c = 0; c < 3; c++) ratios[c] -= min_ratios;

      // Log tone-mapping
      norm = log_tonemapping(norm, data->grey_source, data->black_source, data->dynamic_range);

      // Get the desaturation value based on the log value
      const float desaturation = filmic_desaturate(norm, data->sigma_toe, data->sigma_shoulder, data->saturation);

      for(int c = 0; c < 3; c++) ratios[c] *= norm;

      const float lum = (work_profile) ? dt_ioppr_get_rgb_matrix_luminance(ratios,
                                                                           work_profile->matrix_in,
                                                                           work_profile->lut_in,
                                                                           work_profile->unbounded_coeffs_in,
                                                                           work_profile->lutsize,
                                                                           work_profile->nonlinearlut)
                                        : dt_camera_rgb_luminance(ratios);

      // Desaturate on the non-linear parts of the curve and save ratios
      for(int c = 0; c < 3; c++) ratios[c] = linear_saturation(ratios[c], lum, desaturation) / norm;

      // Filmic S curve on the max RGB
      // Apply the transfer function of the display
      norm = powf(clamp_simd(filmic_spline(norm, spline.M1, spline.M2, spline.M3, spline.M4, spline.M5, spline.latitude_min, spline.latitude_max)), data->output_power);

      // Re-apply ratios
      for(int c = 0; c < 3; c++) pix_out[c] = ratios[c] * norm;
    }
  }

  if(piece->pipe->mask_display & DT_DEV_PIXELPIPE_DISPLAY_MASK)
    dt_iop_alpha_copy(ivoid, ovoid, roi_out->width, roi_out->height);
}