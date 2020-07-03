#!/bin/bash
if [[ -z "${OIDN_DIR}" ]]; then
    echo "OIDN_DIR is not set. Please set this to where you installed/unpacked Intel® Open Image Denoise."
    exit
fi
install_name_tool -id "@loader_path/libOpenImageDenoise.dylib" $OIDN_DIR/lib/libOpenImageDenoise.dylib &&
install_name_tool -id "@loader_path/libtbb.dylib" $OIDN_DIR/lib/libtbb.dylib &&
install_name_tool -id "@loader_path/libtbbmalloc.dylib" $OIDN_DIR/lib/libtbbmalloc.dylib &&
install_name_tool -change "@rpath/libtbb.dylib" "@loader_path/libtbb.dylib" $OIDN_DIR/lib/libOpenImageDenoise.dylib &&
install_name_tool -change "@rpath/libtbbmalloc.dylib" "@loader_path/libtbbmalloc.dylib" $OIDN_DIR/lib/libOpenImageDenoise.dylib &&
cargo build --release &&
mkdir -p target/release/display &&
cp $OIDN_DIR/lib/libOpenImageDenoise.dylib target/release/display/ &&
cp $OIDN_DIR/lib/libtbbmalloc.dylib target/release/display/ &&
cp $OIDN_DIR/lib/libtbb.dylib target/release/display/ &&
mv target/release/libr_display.dylib target/release/display/r-display.dpy &&
echo &&
echo "Display driver is in target/release/display/." &&
echo "Copy the contents of this folder to $DELIGHT/displays/ to use. E.g.:"
echo "sudo cp -R target/release/display/ $DELIGHT/displays/"
