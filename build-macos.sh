#!/bin/bash
BUILD_TYPE="release"
if [[ -z "${OIDN_DIR}" ]]; then
    echo "OIDN_DIR is not set. Please set this to where you installed/unpacked Intel® Open Image Denoise."
    exit
fi
install_name_tool -id "@loader_path/libOpenImageDenoise.dylib" $OIDN_DIR/lib/libOpenImageDenoise.dylib &&
install_name_tool -id "@loader_path/libtbb.dylib" $OIDN_DIR/lib/libtbb.dylib &&
install_name_tool -change "@rpath/libtbb.12.dylib" "@loader_path/libtbb.dylib" $OIDN_DIR/lib/libOpenImageDenoise.dylib &&
if [[ "$BUILD_TYPE" == "release" ]]; then
    cargo build --release
else
    cargo build
fi &&
mkdir -p target/$BUILD_TYPE/display &&
cp $OIDN_DIR/lib/libOpenImageDenoise.dylib target/$BUILD_TYPE/display/ &&
cp $OIDN_DIR/lib/libtbb.dylib target/$BUILD_TYPE/display/ &&
mv target/$BUILD_TYPE/libr_display.dylib target/$BUILD_TYPE/display/r-display.dpy &&
echo &&
echo "Display driver is in target/$BUILD_TYPE/display/." &&
echo "Copy the contents of this folder to $DELIGHT/displays/ to use. E.g.:"
echo "sudo cp -R target/$BUILD_TYPE/display/ $DELIGHT/displays/"
