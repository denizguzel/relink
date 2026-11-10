//! Generates the message catalogs from `locales/` and, for Windows, embeds
//! the executable's icon and version info.

use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    rune_i18n::codegen::generate("locales".as_ref(), &out).expect("generate message catalogs");
    println!("cargo:rerun-if-changed=assets/brand/relink.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rustc-env=RELINK_WINDOWS_APP_ID={}", windows_app_id());

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

/// `windows-app-id` from `[package.metadata.relink]` in Cargo.toml: the
/// installer's AppId, written only there. Checked to be a plain GUID.
fn windows_app_id() -> String {
    let manifest = fs::read_to_string("Cargo.toml").expect("read Cargo.toml");
    let id = manifest
        .lines()
        .find_map(|l| l.trim().strip_prefix("windows-app-id"))
        .and_then(|rest| rest.split('"').nth(1))
        .expect("windows-app-id in [package.metadata.relink]")
        .to_string();
    let groups: Vec<&str> = id.split('-').collect();
    let ok = groups.iter().map(|g| g.len()).eq([8, 4, 4, 4, 12])
        && id.chars().all(|c| c == '-' || c.is_ascii_hexdigit());
    assert!(ok, "windows-app-id is not a GUID: {id}");
    id
}
