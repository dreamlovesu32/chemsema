use crate::*;

pub(crate) fn default_recent_store_path() -> Option<PathBuf> {
    dirs::data_dir().map(|path| {
        path.join("Chemcore")
            .join("desktop")
            .join("recent-files.json")
    })
}

pub(crate) fn load_recent_files(path: &Path) -> Vec<DesktopRecentFile> {
    let Ok(json) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(store) = serde_json::from_str::<RecentFilesStore>(&json) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in store.files {
        if entry.path.trim().is_empty()
            || files
                .iter()
                .any(|existing: &DesktopRecentFile| paths_equal(&existing.path, &entry.path))
        {
            continue;
        }
        let path = PathBuf::from(&entry.path);
        files.push(DesktopRecentFile {
            file_name: if entry.file_name.trim().is_empty() {
                file_name_for_path(&path)
            } else {
                entry.file_name
            },
            exists: path.is_file(),
            path: entry.path,
        });
        if files.len() >= MAX_RECENT_FILES {
            break;
        }
    }
    files
}
