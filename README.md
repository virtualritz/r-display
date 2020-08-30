# r-display

[NSI](https://nsi.readthedocs.io/)/[RenderMan®](https://renderman.pixar.com/)
[OpenEXR](http://www.openexr.com/) display driver written in Rust.

Build instructions should work for **Linux** and **macOS**. On **Windows** your
mileage my vary.

## Building

The [`ndspy-sys`](https://github.com/virtualritz/r-display/blob/master/ndspy-sys/) crate which is part of this project uses the `DELIGHT` environment variable to find the needed display driver API headers. If you have 3Delight installed this will *just* work.

You need a copy of Intel® Open Image Denoise (IOID). Grab a package from their [download section](https://www.openimagedenoise.org/downloads.html). Unpack this somewhere.

### Linux

Export the OIDN location for the build to find the headers & libraries. For example:
```
export OIDN_DIR=$HOME/Downloads/oidn-1.2.1.x86_64.macos/
```

Build the display driver:
```shell
./build-linux.sh
```

### macOS

Export the OIDN location for the build to find the headers & libraries. For example:
```
export OIDN_DIR=$HOME/Downloads/oidn-1.2.1.x86_64.macos/
```

Build the display driver:
```shell
./build-macos.sh
```

## Denoising

The display driver uses [Intel® Open Image Denoise](https://www.openimagedenoise.org/) to denoise the 1st set of RGB channels. This is **switched on by default**. Use the`denoise` (`int`) parameter to control this. Setting this to **zero** switches denoising *off*.

If you want to use **albedo** and **normal** (requires the former) layers to improve the denoising you need to add support for outputting `albedo` from your OSL shaders.

For example if `albedo` contains the albedo add sth. like this to your OSL shader:
```glsl
if( raytype("camera") )	{
    outColor += debug( "albedo" ) * albedo;
}
```

## Compression

This display driver supports the following OpenEXR compression methods which are set by the `compression` (`string`) parameter:

-   [x] `none` uncompressed
-   [x] `zip` (lossless)
-   [x] `rle` (lossless)
-   [x] `piz` (lossless)
-   [x] `pxr24` (lossy)
-   [ ] `b44`, `b44a` not yet supported
-   [ ] `dwaa`, `dwab` not yet supported

## Other parameters

When `premultiply` (`integer`) is set to **zero** the image will be written out *unpremultiplied*.

A `line_order` parameter can be used to set this explicitly to e.g. store the image bottom-top. Accepted values are `increasing` and `decreasing`.
If unspecified the driver will choose a line order matching the compression.

A `tile_size` (`integer[2]`) parameter can be specified to set the width and hight of the tiles the image is stored in.
If unspecified the driver will choose a tile size matching the compression.

## Testing

The code in the `python_test` folder requires [3Delight](https://www.3delight.com/)
to run.
