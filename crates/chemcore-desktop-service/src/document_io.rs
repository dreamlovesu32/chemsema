use crate::*;

impl DesktopDocumentService {
    pub fn read_document_file<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<DesktopOpenedDocument, String> {
        let path = normalize_path(path)?;
        let bytes = fs::read(&path)
            .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
        let format = document_format_for_path_and_bytes(&path, &bytes);
        let text = if format == "ccjz" {
            decompress_gzip_text(&bytes)?
        } else if format == "cdx" {
            cdx_to_cdxml(&bytes)?
        } else {
            String::from_utf8(bytes).map_err(|error| {
                format!("Failed to read {} as UTF-8 text: {error}", path.display())
            })?
        };
        let text = if is_ole_edit_path(&path) {
            ole_edit_document_text(&text).unwrap_or(text)
        } else {
            text
        };
        let format = if format == "text" && looks_like_cdxml(&text) {
            "cdxml".to_string()
        } else if format == "text" {
            "ccjs".to_string()
        } else {
            format
        };
        let opened = DesktopOpenedDocument {
            file_name: file_name_for_path(&path),
            path: path_to_string(&path),
            format,
            text,
        };
        if !is_ole_edit_path(&path) {
            self.add_recent_file(path);
        }
        Ok(opened)
    }

    pub fn write_document_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        content: &str,
        format: Option<&str>,
    ) -> Result<DesktopSavedDocument, String> {
        let path = normalize_path(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("Failed to create directory {}: {error}", parent.display())
            })?;
        }
        let format = format
            .map(normalize_document_format)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| document_format_for_path(&path));
        if format == "ccjz" {
            let bytes = compress_gzip_text(content)?;
            fs::write(&path, bytes)
                .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        } else if format == "cdx" {
            let bytes = cdxml_to_cdx(content)?;
            fs::write(&path, bytes)
                .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        } else {
            fs::write(&path, content)
                .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
        }
        self.add_recent_file(path.clone());
        Ok(DesktopSavedDocument {
            file_name: file_name_for_path(&path),
            path: path_to_string(&path),
            format,
        })
    }

    pub fn recent_files(&self) -> Vec<DesktopRecentFile> {
        self.recent_files
            .iter()
            .map(|entry| DesktopRecentFile {
                path: entry.path.clone(),
                file_name: entry.file_name.clone(),
                exists: Path::new(&entry.path).is_file(),
            })
            .collect()
    }

    pub fn clear_recent_files(&mut self) -> Result<(), String> {
        self.recent_files.clear();
        self.save_recent_files()
    }
    fn add_recent_file(&mut self, path: PathBuf) {
        let path_string = path_to_string(&path);
        self.recent_files
            .retain(|entry| !paths_equal(&entry.path, &path_string));
        self.recent_files.insert(
            0,
            DesktopRecentFile {
                file_name: file_name_for_path(&path),
                path: path_string,
                exists: path.is_file(),
            },
        );
        self.recent_files.truncate(MAX_RECENT_FILES);
        let _ = self.save_recent_files();
    }

    fn save_recent_files(&self) -> Result<(), String> {
        let Some(path) = &self.recent_store_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Failed to create recent-file directory {}: {error}",
                    parent.display()
                )
            })?;
        }
        let store = RecentFilesStore {
            files: self.recent_files(),
        };
        let json = serde_json::to_string_pretty(&store).map_err(|error| error.to_string())?;
        fs::write(path, format!("{json}\n"))
            .map_err(|error| format!("Failed to write {}: {error}", path.display()))
    }
}
