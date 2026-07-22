#[cfg(target_os = "windows")]
mod windows_office;

#[cfg(target_os = "windows")]
pub fn run() -> Result<(), String> {
    windows_office::run()
}

#[cfg(not(target_os = "windows"))]
pub fn run() -> Result<(), String> {
    Err("chemsema-office is currently only supported on Windows.".to_string())
}

#[cfg(target_os = "windows")]
pub fn write_emf_payload_json(
    output_path: impl AsRef<std::path::Path>,
    payload_json: &str,
) -> Result<(), String> {
    windows_office::write_emf_payload_json(output_path.as_ref(), payload_json)
}

#[cfg(not(target_os = "windows"))]
pub fn write_emf_payload_json(
    _output_path: impl AsRef<std::path::Path>,
    _payload_json: &str,
) -> Result<(), String> {
    Err("EMF export is only available on Windows.".to_string())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn writes_a_valid_enhanced_metafile() {
        let document_json = serde_json::to_string(&chemsema_engine::ChemSemaDocument::blank())
            .expect("blank document should serialize");
        let payload = serde_json::json!({
            "chemsemaDocumentJson": document_json,
            "renderListJson": null,
            "svg": null,
        });
        let output =
            std::env::temp_dir().join(format!("chemsema-emf-export-{}.emf", std::process::id()));
        write_emf_payload_json(&output, &payload.to_string()).expect("EMF should be written");
        let bytes = std::fs::read(&output).expect("EMF should be readable");
        let _ = std::fs::remove_file(output);
        assert!(bytes.len() >= 88, "EMF header should be present");
        assert_eq!(&bytes[40..44], b" EMF", "EMF signature should match");
    }
}
