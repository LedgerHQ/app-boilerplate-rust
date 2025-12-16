use image::{ImageFormat, ImageReader, Pixel};

fn generate_install_parameters() {
    // Get cargo metadata
    let output = std::process::Command::new("cargo")
        .args(&["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("Failed to execute cargo metadata");

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Failed to parse cargo metadata output");
    let metadata_ledger = &metadata["packages"][0]["metadata"]["ledger"];

    // Fill APP_NAME environment variable (stored in ledger.app_name section in the ELF (see info.rs))
    let app_name = metadata_ledger["name"].as_str().expect("name not found");
    println!("cargo:rustc-env=APP_NAME={}", app_name);
    println!("cargo:warning=APP_NAME is {}", app_name);

    // Fill APP_FLAGS environment variable (stored in ledger.app_flags section in the ELF (see info.rs))
    let app_flags = metadata_ledger["flags"].as_str().expect("flags not found");
    println!("cargo:rustc-env=APP_FLAGS={}", app_flags);
    println!("cargo:warning=APP_FLAGS is {}", app_flags);

    // Generate install_params TLV blob (stored as install_parameters symbol in the ELF (see info.rs))
    let app_version = env!("CARGO_PKG_VERSION");
    println!("cargo:warning=app_version is {}", app_version);
    let curves = metadata_ledger["curve"]
        .as_array()
        .expect("curves not found")
        .iter()
        .map(|v| format!("{}", v.as_str().unwrap()))
        .collect::<Vec<_>>();
    println!("cargo:warning=curves are {:x?}", curves);
    let paths = metadata_ledger["path"]
        .as_array()
        .expect("paths not found")
        .iter()
        .map(|v| format!("{}", v.as_str().unwrap()))
        .collect::<Vec<_>>();
    println!("cargo:warning=paths are {:x?}", paths);
    let mut paths_slip21: Vec<String> = Vec::default();
    match metadata_ledger["path_slip21"] {
        serde_json::Value::Null => {}
        _ => {
            paths_slip21 = metadata_ledger["path_slip21"]
                .as_array()
                .expect("paths_slip21 not found")
                .iter()
                .map(|v| format!("{}", v.as_str().unwrap()))
                .collect::<Vec<_>>();
        }
    }

    let install_params_exe = match std::env::var("LEDGER_SDK_PATH") {
        Ok(path) => format!("{}/install_params.py", path),
        Err(_) => format!(
            "/opt/{}-secure-sdk/install_params.py",
            std::env::var_os("CARGO_CFG_TARGET_OS")
                .unwrap()
                .to_str()
                .unwrap()
        ),
    };
    let mut generate_tlv_install_params = std::process::Command::new("python3");
    generate_tlv_install_params.arg(install_params_exe.as_str());
    generate_tlv_install_params.arg("--appName").arg(app_name);
    generate_tlv_install_params
        .arg("--appVersion")
        .arg(app_version);
    curves.iter().for_each(|p| {
        generate_tlv_install_params.arg("--curve").arg(p.as_str());
    });
    paths.iter().for_each(|p| {
        generate_tlv_install_params.arg("--path").arg(p.as_str());
    });
    paths_slip21.iter().for_each(|p| {
        generate_tlv_install_params
            .arg("--path_slip21")
            .arg(p.as_str());
    });
    let output = generate_tlv_install_params
        .output()
        .expect("Failed to execute install_params_generator");

    let tlv_blob = format!(
        "[{}]",
        std::str::from_utf8(output.stdout.as_slice())
            .unwrap()
            .trim()
    );

    // Parse the TLV blob and create temp txt files for inclusion (see info.rs)
    let bytes: Vec<u8> = tlv_blob
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                u8::from_str_radix(trimmed.trim_start_matches("0x"), 16).ok()
            }
        })
        .collect();

    let byte_array_str = bytes
        .iter()
        .map(|b| format!("0x{:02x}", b))
        .collect::<Vec<_>>()
        .join(",");

    // Write to files in OUT_DIR for inclusion
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // Write the array with brackets for direct inclusion
    std::fs::write(
        std::path::Path::new(&out_dir).join("install_params.txt"),
        format!("[{}]", byte_array_str),
    )
    .unwrap();

    std::fs::write(
        std::path::Path::new(&out_dir).join("install_params_len.txt"),
        bytes.len().to_string(),
    )
    .unwrap();

    println!("cargo:warning=INSTALL_PARAMS_BYTES is [{}]", byte_array_str);
    println!("cargo:warning=INSTALL_PARAMS_LEN is {}", bytes.len());
}

fn generate_home_nano_nbgl_glyph() {
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

fn main() {
    println!("cargo:rerun-if-changed=script.ld");
    println!("cargo:rerun-if-changed=icons/crab_14x14.gif");
    println!("cargo:rerun-if-changed=icons/mask_14x14.gif");
    println!("cargo:rerun-if-changed=Cargo.toml");

    generate_install_parameters();
    generate_home_nano_nbgl_glyph();
}
