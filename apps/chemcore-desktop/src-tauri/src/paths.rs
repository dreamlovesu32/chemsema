use crate::*;

pub(crate) fn document_file_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new()
        .add_filter(
            "Chemcore, ChemDraw, and SDF",
            &["ccjz", "ccjs", "cdxml", "cdx", "sdf", "sd"],
        )
        .add_filter("Chemcore compressed", &["ccjz"])
        .add_filter("Chemcore JSON", &["ccjs"])
        .add_filter("ChemDraw CDXML", &["cdxml"])
        .add_filter("ChemDraw CDX", &["cdx"])
        .add_filter("MDL SDfile", &["sdf", "sd"])
        .add_filter("SVG", &["svg"])
}

pub(crate) fn normalize_output_path(path: String) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty() {
        return Err("Path is empty.".to_string());
    }
    if path.is_absolute() {
        Ok(path)
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|error| error.to_string())
    }
}
pub(crate) fn startup_file_args() -> Vec<String> {
    let cwd = std::env::current_dir().ok();
    openable_document_args(std::env::args().skip(1), cwd.as_deref())
}

pub(crate) fn openable_document_args<I, S>(args: I, cwd: Option<&Path>) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .filter_map(|arg| resolve_open_arg(arg.as_ref(), cwd))
        .collect()
}

pub(crate) fn resolve_open_arg(arg: &str, cwd: Option<&Path>) -> Option<String> {
    let trimmed = arg.trim_matches('"');
    if !is_openable_document_arg(trimmed) {
        return None;
    }
    let path = PathBuf::from(trimmed);
    let path = if path.is_absolute() {
        path
    } else {
        cwd.unwrap_or_else(|| Path::new(".")).join(path)
    };
    Some(path.to_string_lossy().to_string())
}

pub(crate) fn is_openable_document_arg(arg: &str) -> bool {
    let lower = arg.to_ascii_lowercase();
    lower.ends_with(".ccjz")
        || lower.ends_with(".ccjs")
        || lower.ends_with(".cdxml")
        || lower.ends_with(".cdx")
        || lower.ends_with(".svg")
}
