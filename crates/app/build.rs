// Embeds the brand's icon and names into the Windows executable.

use std::path::PathBuf;

#[allow(dead_code)]
mod config {
    include!("../brand/config.rs");
}

fn main() -> Result<(), String> {
    println!("cargo:rerun-if-env-changed=KG_BRAND_DIR");
    println!("cargo:rerun-if-changed=../brand/config.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?);
    let brand = config::load_brand(&manifest.join("..").join(".."))?;
    let ico = brand.asset(&brand.file.assets.icon_ico);
    println!(
        "cargo:rerun-if-changed={}",
        brand.dir.join("brand.toml").display()
    );
    println!("cargo:rerun-if-changed={}", ico.display());

    let version = std::env::var("CARGO_PKG_VERSION").map_err(|e| e.to_string())?;
    let numbers: Vec<&str> = version.split(['.', '-', '+']).take(3).collect();
    let numeric = format!("{},0", numbers.join(","));
    // Strings in resource scripts double their quotes.
    let quoted = |s: &str| s.replace('"', "\"\"");
    let rc = format!(
        "#pragma code_page(65001)\n\
         1 ICON \"{ico}\"\n\
         1 VERSIONINFO\n\
         FILEVERSION {numeric}\n\
         PRODUCTVERSION {numeric}\n\
         FILEOS 0x40004\n\
         FILETYPE 0x1\n\
         BEGIN\n\
         BLOCK \"StringFileInfo\"\n\
         BEGIN\n\
         BLOCK \"040904B0\"\n\
         BEGIN\n\
         VALUE \"CompanyName\", \"{company}\"\n\
         VALUE \"FileDescription\", \"{name}\"\n\
         VALUE \"ProductName\", \"{name}\"\n\
         VALUE \"FileVersion\", \"{version}\"\n\
         VALUE \"ProductVersion\", \"{version}\"\n\
         END\n\
         END\n\
         BLOCK \"VarFileInfo\"\n\
         BEGIN\n\
         VALUE \"Translation\", 0x409, 1200\n\
         END\n\
         END\n",
        ico = ico.display().to_string().replace('\\', "\\\\"),
        company = quoted(&brand.file.company.name),
        name = quoted(&brand.file.app.name),
    );
    let out = PathBuf::from(std::env::var("OUT_DIR").map_err(|e| e.to_string())?).join("app.rc");
    std::fs::write(&out, rc).map_err(|e| e.to_string())?;
    if let Err(err) = embed_resource::compile(&out, embed_resource::NONE).manifest_optional() {
        println!("cargo:warning=could not embed the app icon: {err}");
    }
    Ok(())
}
