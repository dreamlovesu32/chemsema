use crate::*;

pub(crate) fn build_native_menu(
    app: &tauri::AppHandle,
    recent_files: &[DesktopRecentFile],
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let new_item = MenuItemBuilder::with_id("desktop-file-new", "&New")
        .accelerator("Ctrl+N")
        .build(app)?;
    let open_item = MenuItemBuilder::with_id("desktop-file-open", "&Open...")
        .accelerator("Ctrl+O")
        .build(app)?;
    let save_item = MenuItemBuilder::with_id("desktop-file-save", "&Save")
        .accelerator("Ctrl+S")
        .build(app)?;
    let save_as_item = MenuItemBuilder::with_id("desktop-file-save-as", "Save &As...")
        .accelerator("Ctrl+Shift+S")
        .build(app)?;
    let export_cdxml_item =
        MenuItemBuilder::with_id("desktop-file-export-cdxml", "Export &CDXML...").build(app)?;
    let export_svg_item =
        MenuItemBuilder::with_id("desktop-file-export-svg", "Export &SVG...").build(app)?;
    let export_pdf_item =
        MenuItemBuilder::with_id("desktop-file-export-pdf", "Export &PDF Preview...").build(app)?;
    let export_emf_item =
        MenuItemBuilder::with_id("desktop-file-export-emf", "Export &EMF...").build(app)?;
    let clear_recent_item =
        MenuItemBuilder::with_id("desktop-recent-clear", "&Clear Recent Files").build(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("E&xit"))?;

    let mut recent_builder = SubmenuBuilder::new(app, "&Recent Files");
    if recent_files.is_empty() {
        let empty_item = MenuItemBuilder::with_id("desktop-recent-empty", "No Recent Files")
            .enabled(false)
            .build(app)?;
        recent_builder = recent_builder.item(&empty_item);
    } else {
        for (index, file) in recent_files.iter().enumerate() {
            let label = format!("{} {}", index + 1, file.file_name);
            let item = MenuItemBuilder::with_id(format!("desktop-recent-open-{index}"), label)
                .build(app)?;
            recent_builder = recent_builder.item(&item);
        }
        recent_builder = recent_builder.separator().item(&clear_recent_item);
    }
    let recent_menu = recent_builder.build()?;

    let undo_item = MenuItemBuilder::with_id("desktop-edit-undo", "&Undo")
        .accelerator("Ctrl+Z")
        .build(app)?;
    let redo_item = MenuItemBuilder::with_id("desktop-edit-redo", "&Redo")
        .accelerator("Ctrl+Y")
        .build(app)?;
    let cut_item = MenuItemBuilder::with_id("desktop-edit-cut", "Cu&t")
        .accelerator("Ctrl+X")
        .build(app)?;
    let copy_item = MenuItemBuilder::with_id("desktop-edit-copy", "&Copy")
        .accelerator("Ctrl+C")
        .build(app)?;
    let paste_item = MenuItemBuilder::with_id("desktop-edit-paste", "&Paste")
        .accelerator("Ctrl+V")
        .build(app)?;
    let delete_item = MenuItemBuilder::with_id("desktop-edit-delete", "&Delete")
        .accelerator("Delete")
        .build(app)?;

    let zoom_in_item = MenuItemBuilder::with_id("desktop-view-zoom-in", "Zoom &In")
        .accelerator("Ctrl+=")
        .build(app)?;
    let zoom_out_item = MenuItemBuilder::with_id("desktop-view-zoom-out", "Zoom &Out")
        .accelerator("Ctrl+-")
        .build(app)?;
    let fit_item = MenuItemBuilder::with_id("desktop-view-fit", "&Fit All")
        .accelerator("Ctrl+0")
        .build(app)?;

    let file_menu = SubmenuBuilder::new(app, "&File")
        .item(&new_item)
        .item(&open_item)
        .separator()
        .item(&save_item)
        .item(&save_as_item)
        .separator()
        .item(&export_cdxml_item)
        .item(&export_svg_item)
        .item(&export_pdf_item)
        .item(&export_emf_item)
        .separator()
        .item(&recent_menu)
        .separator()
        .item(&quit_item)
        .build()?;
    let edit_menu = SubmenuBuilder::new(app, "&Edit")
        .item(&undo_item)
        .item(&redo_item)
        .separator()
        .item(&cut_item)
        .item(&copy_item)
        .item(&paste_item)
        .item(&delete_item)
        .build()?;
    let view_menu = SubmenuBuilder::new(app, "&View")
        .item(&zoom_in_item)
        .item(&zoom_out_item)
        .item(&fit_item)
        .build()?;

    MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .build()
}

pub(crate) fn refresh_native_menu(app: &tauri::AppHandle) {
    if !USE_NATIVE_MENU {
        return;
    }
    let recent_files = app
        .try_state::<DesktopState>()
        .and_then(|state| {
            state
                .service
                .lock()
                .ok()
                .map(|service| service.recent_files())
        })
        .unwrap_or_default();
    if let Ok(menu) = build_native_menu(app, &recent_files) {
        let _ = app.set_menu(menu);
    }
}

pub(crate) fn install_native_menu(app: &tauri::App) -> tauri::Result<()> {
    if !USE_NATIVE_MENU {
        return Ok(());
    }
    let recent_files = app
        .try_state::<DesktopState>()
        .and_then(|state| {
            state
                .service
                .lock()
                .ok()
                .map(|service| service.recent_files())
        })
        .unwrap_or_default();
    let menu = build_native_menu(app.handle(), &recent_files)?;
    app.set_menu(menu)?;
    Ok(())
}

pub(crate) fn handle_native_menu_event(app: &tauri::AppHandle, id: &str) {
    if let Some(index) = id.strip_prefix("desktop-recent-open-") {
        if let Ok(index) = index.parse::<usize>() {
            let path = app
                .try_state::<DesktopState>()
                .and_then(|state| {
                    state
                        .service
                        .lock()
                        .ok()
                        .and_then(|service| service.recent_files().get(index).cloned())
                })
                .map(|entry| entry.path);
            if let Some(path) = path {
                emit_open_paths(app, vec![path]);
            }
        }
        return;
    }

    if id == "desktop-recent-clear" {
        if let Some(state) = app.try_state::<DesktopState>() {
            if let Ok(mut service) = state.service.lock() {
                let _ = service.clear_recent_files();
            }
        }
        refresh_native_menu(app);
        return;
    }

    let command = match id {
        "desktop-file-new" => "new",
        "desktop-file-open" => "open",
        "desktop-file-save" => "save",
        "desktop-file-save-as" => "save-as",
        "desktop-file-export-cdxml" => "save-cdxml",
        "desktop-file-export-svg" => "save-svg",
        "desktop-file-export-pdf" => "save-pdf",
        "desktop-file-export-emf" => "save-emf",
        "desktop-edit-undo" => "undo",
        "desktop-edit-redo" => "redo",
        "desktop-edit-cut" => "cut",
        "desktop-edit-copy" => "copy",
        "desktop-edit-paste" => "paste",
        "desktop-edit-delete" => "delete",
        "desktop-view-zoom-in" => "zoom-in",
        "desktop-view-zoom-out" => "zoom-out",
        "desktop-view-fit" => "fit",
        _ => return,
    };
    emit_menu_command_to_focused(app, command);
}

pub(crate) fn emit_menu_command_to_focused(app: &tauri::AppHandle, command: &str) {
    let payload = DesktopMenuPayload {
        command: command.to_string(),
    };
    let focused = app
        .webview_windows()
        .into_values()
        .find(|window| window.is_focused().unwrap_or(false))
        .or_else(|| app.get_webview_window("main"))
        .or_else(|| app.webview_windows().into_values().next());
    if let Some(window) = focused {
        let _ = window.emit(EVENT_DESKTOP_MENU, payload);
    }
}
