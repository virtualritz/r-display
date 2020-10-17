# r-display

[NSI](https://nsi.readthedocs.io/)/[RenderMan®](https://renderman.pixar.com/)
[OpenEXR](http://www.openexr.com/) display driver with Open Image
Denoise support written in Rust.

Build instructions should work for **Linux** and **macOS**. On **Windows** your
mileage my vary.

## Building

You need a copy of Intel® Open Image Denoise (IOID). Grab a package
from their
[download section](https://www.openimagedenoise.org/downloads.html).
Unpack this somewhere. We refer to this below as the *OIDN location*.

### Linux

Export the OIDN location for the build to find the headers & libraries.
For example:
```shell
export OIDN_DIR=$HOME/Downloads/oidn-1.2.4.x86_64.linux/
```

Build the display driver:
```shell
./build-linux.sh
```

### macOS

Export the OIDN location for the build to find the headers & libraries.
For example:
```shell
export OIDN_DIR=$HOME/Downloads/oidn-1.2.4.x86_64.macos/
```

Build the display driver:
```shell
./build-macos.sh
```

### Windows

#### Brief

We assume you are using PowerShell. You need to have LLVM installed
somewhere.
Export the OIDN location for the build to find the headers & libraries.
For example (using PowerShell):
```powershell
set OIDN_DIR='C:\Downloads\oidn-1.2.4.x64.vc14.windows\oidn-1.2.4.x64.vc14.windows'
```
Set the he LLVM installation directory. For example:
```powershell
$Env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin\"
```
Build the display driver:
```powershell
cargo build --release
```

### Detailed (For Non-Technical Peeps)

You need to have PowerShell 2 or later installed. To install it click
the lower left corner Windows icon and start typing `PowerShell`.
[Install Chocolatey](https://chocolatey.org/install). It is a package
manager that will make installing the rest of the stuff you need painless.

Open PowerShell and follow the instructions.

#. Install `git`:
```powershell
choco install git
```
#. Install [Visual C++ Build Tools](https://visualstudio.microsoft.com/ru/visual-cpp-build-tools/):
```powershell
choco install visualcpp-build-tools
```
#. Install [Rust](https://www.rust-lang.org/):
```powershell
choco install rust-ms
```
#. Install [LLVM](https://llvm.org/):
```powershell
choco install llvm
```
#. Set the LLVM location:
```powershell
$Env:LIBCLANG_PATH = "C:\ProgramData\Chocolatey\bin"
```
#. Set the OIDN location you choose [above](##Building). For example:
```powershell
$Env:IDN_DIR = "C:\Downloads\oidn-1.2.4.x64.vc14.windows\oidn-1.2.4.x64.vc14.windows"
```
#. Create some folder to host the repository during build and change to
there. Note that you can delete this later.
```powershell
md -Path "C:\MyProjects"
cd "C:\MyProjects"
```
#. Clone the r-display repository cand hop into it:
```powershell
git clone https://github.com/virtualritz/r-display
cd r-display
```
Build the display driver:
```powershell
cargo build --release
```
#. Copy the result to the 3Delight display folder:
```powershell
cp target\release\r-display.dll $Env:DELIGHT\displays
```

## How To

There is an example app in `examples/denoise.rs`. This shows how to add
the two optional auxiliary AOVs for albedo & normal when instancing the
display driver through the [NSI crate](https://crates.io/crates/nsi).

For this to work you need to download & install a
[3Delight](https://www.3delight.com/) package for your platform.

To run the example:
```shell
cargo run --example denoise
```

This will launch the render and dump the raw data of a render with
*one* sample per pixel to 3Delight Display. It will save a denoised
version to `test_0001samples.exr`.

## Parameters

### Denoising

![Comparispon of denoising results|ɴsɪ](test.jpg)

The display driver uses
[Intel® Open Image Denoise](https://www.openimagedenoise.org/)
to denoise the 1st set of RGB channels. This is **switched on by
default**.

Use the`denoise` (`float`) parameter to control this.

Setting this to **zero** switches denoising *off*.

Setting this to a value above *0* and below *1* linearly blends then
denoised image with the original.

Setting it to **one** (or above) switches denosing on. This means the
original pixels will be discarded and replaced with denoised ones.

If you want support for keeping the original image in a separate layer
of the EXR open an issue and I see what can be done.

If you want to use **albedo** and **normal** (requires the former)
layers to improve the denoising you need to add support for outputting
`albedo` from your OSL shaders.

For example if `albedo` contains the albedo add sth. like this to your
OSL shader:
```glsl
if( raytype("camera") )	{
    outColor += debug("albedo") * albedo;
}
```

### Compression

This display driver supports the following OpenEXR compression methods
which are set by the `compression` (`string`) parameter:

-   [x] `none` uncompressed
-   [x] `zip` (lossless)
-   [x] `rle` (lossless)
-   [x] `piz` (lossless)
-   [x] `pxr24` (lossy)
-   [ ] `b44`, `b44a` not yet supported
-   [ ] `dwaa`, `dwab` not yet supported

### Other

When `premultiply` (`integer`) is set to **zero** the image will be
written out *unpremultiplied*.

A `line_order` parameter can be used to set this explicitly to e.g.
store the image bottom-top. Accepted values are `increasing` and
`decreasing`. If unspecified the driver will choose a line order
matching the compression.

A `tile_size` (`integer[2]`) parameter can be specified to set the
width and height of the tiles the image is stored in.
If unspecified the driver will choose a tile size matching the
compression.

## Caveats

The display driver needs work if one wanted to use it for multi-layer
EXRs (e.g. writing a single EXR that contains a bunch of AOVs as
layers).

What would be needed was a way to designate which input layers should
be denoised and maybe also a way to filter out utility passes only
added for the denoiser so they don’t take up disk space.

If you want to use this in production and need those features ping me.

### Metadata

The display driver exports some metadata that is common to EXR files:

-   [x] `pixel aspect`
-   [x] `world to camera`
-   [x] `world to normalized device`
-   [x] `near clip plane`
-   [x] `far clip plane`
-   [x] `software name`
