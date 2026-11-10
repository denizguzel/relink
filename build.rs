//! Generates the message catalogs from `locales/` and, for Windows, embeds
//! the executable's icon and version info.

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    rune_i18n::codegen::generate("locales".as_ref(), &out).expect("generate message catalogs");
    println!("cargo:rerun-if-changed=assets/brand/relink.ico");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let ico =
            PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets/brand/relink.ico");
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
      VALUE "FileDescription", "Re-Link: keeps browsers connected on a VPN split tunnel"
      VALUE "ProductName", "Re-Link"
      VALUE "InternalName", "relink"
      VALUE "LegalCopyright", "Re-Link contributors"
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
