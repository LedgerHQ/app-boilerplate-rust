use image::{ImageFormat, ImageReader, Pixel};


fn generate_install_parameters() {
    
    // call cargo metadata to get ledger.metadata
    let output = std::process::Command::new("cargo")
        .args(&["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("Failed to execute cargo metadata");

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("Failed to parse cargo metadata output");

    let metadata_ledger = &metadata["packages"][0]["metadata"]["ledger"];

    let app_name = metadata_ledger["name"].as_str().expect("name not found");
    println!("cargo:rustc-env=APP_NAME={}", app_name);
    println!("cargo:warning=APP_NAME is {}", app_name);

}


fn main() {
    println!("cargo:rerun-if-changed=script.ld");
    println!("cargo:rerun-if-changed=icons/crab_14x14.gif");
    println!("cargo:rerun-if-changed=icons/mask_14x14.gif");

    generate_install_parameters();

    let path = std::path::PathBuf::from("icons");
    let reader = ImageReader::open(path.join("crab_14x14.gif")).unwrap();
    let img = reader.decode().unwrap();
    let mut gray = img.into_luma8();

    // Apply mask
    let mask = ImageReader::open(path.join("mask_14x14.gif"))
        .unwrap()
        .decode()
        .unwrap()
        .into_luma8();

    for (x, y, mask_pixel) in mask.enumerate_pixels() {
        let mask_value = mask_pixel[0];
        let mut gray_pixel = *gray.get_pixel(x, y);
        if mask_value == 0 {
            gray_pixel = image::Luma([0]);
        } else {
            gray_pixel.invert();
        }
        gray.put_pixel(x, y, gray_pixel);
    }

    let glyph_path = std::path::PathBuf::from("glyphs");
    gray.save_with_format(glyph_path.join("home_nano_nbgl.png"), ImageFormat::Png)
        .unwrap();
}
