//! The esp-hal linker script, as in ../device/build.rs. Without it the image has no entry point.
fn main() {
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}
