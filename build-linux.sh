#!/bin/bash
if [[ -z "${OIDN_DIR}" ]]; then
    echo "OIDN_DIR is not set. Please set this to where you installed/unpacked Intel® Open Image Denoise."
    exit
fi
cargo build --release &&
mkdir -p target/release/display &&
cp $OIDN_DIR/lib/libOpenImageDenoise.so target/release/display/ &&
cp $OIDN_DIR/lib/libtbbmalloc.so target/release/display/ &&
cp $OIDN_DIR/lib/libtbb.so target/release/display/ &&
mv target/release/libr_display.so target/release/display/r-display.dpy &&
echo &&
echo "Display driver is in target/release/display/." &&
echo "Copy the contents of this folder to $DELIGHT/displays/ to use. E.g.:"
echo "sudo cp -R target/release/display/ $DELIGHT/displays/"
