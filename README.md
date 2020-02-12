# r-display

Minimal [NSI](https://nsi.readthedocs.io/)/RenderMan 8bit PNG display driver template written in Rust.

The build only works with [3Delight](https://www.3delight.com/) out of the box. Build instructions should work for **Linux** and **macOS**. On **Windows** your mileage my vary.

The [`ndspy-sys`](https://github.com/virtualritz/r-display/blob/master/ndspy-sys/) crate which is part of this project uses the `$DELIGHT` environment variable to find the needed display driver API headers. Edit [`ndspy-sys/build.rs`](https://github.com/virtualritz/r-display/blob/master/ndspy-sys/build.rs) to add (an) additional or different search path(s) for these headers.


## Prequisites (assuming you never used Rust)

Install Rust:
```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Clone the repository.
```
git clone https://github.com/virtualritz/r-display.git
cd r-display
```

## Building

Kick off the build:
```shell
cargo build --release
```

Once this has succeeded, change to the `python_test` folder and symlink the display driver:
```
cd python_test
ln -s ../target/release/libr_display.dylib rdisplay.dpy
```

Now run the test:
```
python lived_edit.py
```

---
**NOTE**

The symlinking step is only needed once.

---
**NOTE**

If you do a debug build (omitting the `--release` flag to `cargo build`), the asset will be in `../target/debug/libr_display.dylib` instead. You will need to change the symbolic link accordingly.

---

