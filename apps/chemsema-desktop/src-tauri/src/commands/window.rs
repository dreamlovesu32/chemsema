use crate::*;

#[tauri::command]
pub(crate) fn desktop_window_set_title(window: WebviewWindow, title: String) -> Result<(), String> {
    let title = title.trim();
    window
        .set_title(if title.is_empty() { "ChemSema" } else { title })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn desktop_window_minimize(window: WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn desktop_window_toggle_maximize(window: WebviewWindow) -> Result<(), String> {
    if window.is_maximized().map_err(|error| error.to_string())? {
        window.unmaximize().map_err(|error| error.to_string())
    } else {
        window.maximize().map_err(|error| error.to_string())
    }
}

#[tauri::command]
pub(crate) fn desktop_window_close(window: WebviewWindow) -> Result<(), String> {
    window.close().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn desktop_window_destroy(window: WebviewWindow) -> Result<(), String> {
    window.destroy().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn desktop_window_start_dragging(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn desktop_window_is_maximized(window: WebviewWindow) -> Result<bool, String> {
    window.is_maximized().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn desktop_window_detach_document(
    app: tauri::AppHandle,
    state: tauri::State<'_, DesktopState>,
    document: DesktopDetachedDocumentPayload,
    screen_x: Option<f64>,
    screen_y: Option<f64>,
) -> Result<String, String> {
    let label = next_document_window_label(&app);
    {
        let mut pending = state
            .pending_detached_documents
            .lock()
            .map_err(|error| error.to_string())?;
        pending.insert(label.clone(), document.clone());
    }
    let title = if document.title.trim().is_empty() {
        "Untitled"
    } else {
        document.title.trim()
    };
    let url = editor_window_url(&app)?;
    let mut builder = WebviewWindowBuilder::new(&app, label.clone(), url)
        .title(title)
        .inner_size(1280.0, 900.0)
        .min_inner_size(960.0, 640.0)
        .resizable(true)
        .decorations(false)
        .shadow(true)
        .drag_and_drop(true)
        .focused(true);
    if let (Some(x), Some(y)) = (screen_x, screen_y) {
        builder = builder.position((x - 240.0).max(0.0), (y - 20.0).max(0.0));
    }
    if let Err(error) = builder.build() {
        if let Ok(mut pending) = state.pending_detached_documents.lock() {
            pending.remove(&label);
        }
        return Err(error.to_string());
    }
    Ok(label)
}

#[tauri::command]
pub(crate) fn desktop_window_take_detached_document(
    window: WebviewWindow,
    state: tauri::State<'_, DesktopState>,
) -> Result<Option<DesktopDetachedDocumentPayload>, String> {
    let mut pending = state
        .pending_detached_documents
        .lock()
        .map_err(|error| error.to_string())?;
    Ok(pending.remove(window.label()))
}
