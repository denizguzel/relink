//! Generates the message catalogs from `locales/` and, for Windows, embeds
//! the executable's icon and version info.

#[path = "src/iconart.rs"]
#[allow(dead_code)]
mod iconart;

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    rune_i18n::codegen::generate("locales".as_ref(), &out).expect("generate message catalogs");
    println!("cargo:rerun-if-changed=src/iconart.rs");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let ico = out.join("relink.ico");
        fs::write(
            &ico,
            iconart::ico(iconart::BLUE, &[16, 24, 32, 48, 64, 256]),
        )
        .unwrap();
        let version = env::var("CARGO_PKG_VERSION").unwrap();
        let rc = out.join("relink.rc");
        fs::write(
            &rc,
            format!(
                r#"1 ICON "{ico}"
1 VERSIONINFO
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "Re-Link"
      VALUE "ProductName", "Re-Link"
      VALUE "ProductVersion", "{version}"
      VALUE "OriginalFilename", "relink.exe"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
                ico = ico.display().to_string().replace('\\', "/")
            ),
        )
        .unwrap();
        embed_resource::compile_for(&rc, ["relink"], embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
