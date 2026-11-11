//! Emits validated build-brand metadata for packaging tools, without shell evaluation.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let field = std::env::args().nth(1).ok_or("missing metadata field")?;
    let value = match field.as_str() {
        // Installed names and folders follow the channel (KG_CHANNEL).
        "name" => kg_brand::INSTALL_NAME,
        "channel" => kg_brand::CHANNEL,
        "data_dir" => kg_brand::DATA_DIR,
        "executable" => kg_brand::EXECUTABLE,
        "slug" => kg_brand::INSTALL_SLUG,
        "company" => kg_brand::COMPANY,
        "url" => kg_brand::COMPANY_URL,
        "repository" => kg_brand::REPOSITORY,
        "icon_png" => kg_brand::ICON_PNG_PATH,
        "icon_ico" => kg_brand::ICON_ICO_PATH,
        "guid" => kg_brand::INSTALLER_GUID,
        "release-ready" => {
            if !kg_brand::LEGAL_REVIEWED || !kg_brand::COMMERCIAL_REVIEWED {
                return Err(
                    "client legal and commercial review are required before release".into(),
                );
            }
            "yes"
        }
        _ => return Err("unknown metadata field".into()),
    };
    println!("{value}");
    Ok(())
}
