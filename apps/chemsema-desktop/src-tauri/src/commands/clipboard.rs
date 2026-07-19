use crate::*;

#[tauri::command]
pub(crate) fn desktop_clipboard_write(
    app: tauri::AppHandle,
    payload: NativeClipboardWritePayload,
) -> Result<(), String> {
    native_clipboard_write(&app, payload)
}

#[tauri::command]
pub(crate) fn desktop_clipboard_read() -> Result<NativeClipboardReadPayload, String> {
    native_clipboard_read()
}

#[cfg(target_os = "windows")]
fn native_clipboard_write(
    app: &tauri::AppHandle,
    payload: NativeClipboardWritePayload,
) -> Result<(), String> {
    use windows_sys::Win32::System::DataExchange::{
        EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock};

    {
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return Err("Failed to open Windows clipboard.".to_string());
            }
        }
        let _guard = ClipboardCloseGuard;

        unsafe {
            if EmptyClipboard() == 0 {
                return Err("Failed to clear Windows clipboard.".to_string());
            }
        }

        let mut wrote = false;
        // Write portable text/vector formats first. Office-specific OLE data is
        // added afterward by the helper process so normal clipboard consumers
        // are not forced through COM.
        for (format_name, value) in [
            (
                FORMAT_CHEMSEMA_FRAGMENT,
                payload.chemsema_fragment_json.as_deref(),
            ),
            (
                FORMAT_CHEMSEMA_DOCUMENT_JSON,
                payload.chemsema_document_json.as_deref(),
            ),
            (FORMAT_CDXML_MIME, payload.cdxml.as_deref()),
            (FORMAT_SVG_MIME, payload.svg.as_deref()),
            (FORMAT_SVG, payload.svg.as_deref()),
        ] {
            let Some(value) = value.filter(|value| !value.is_empty()) else {
                continue;
            };
            let format = unsafe { RegisterClipboardFormatW(wide_null(format_name).as_ptr()) };
            if format == 0 {
                return Err(format!(
                    "Failed to register clipboard format {format_name}."
                ));
            }
            unsafe {
                write_clipboard_utf8(
                    format,
                    value,
                    GlobalAlloc,
                    GlobalLock,
                    GlobalUnlock,
                    SetClipboardData,
                )?;
            }
            wrote = true;
        }

        let text = payload
            .text
            .as_deref()
            .or(payload.cdxml.as_deref())
            .filter(|value| !value.is_empty());
        if let Some(text) = text {
            unsafe {
                write_clipboard_utf16(
                    13,
                    text,
                    GlobalAlloc,
                    GlobalLock,
                    GlobalUnlock,
                    SetClipboardData,
                )?;
            }
            wrote = true;
        }

        if !wrote {
            return Err("Clipboard payload is empty.".to_string());
        }
    }

    if payload
        .chemsema_document_json
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        native_office_ole_clipboard_write(app, &payload)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn native_office_ole_clipboard_write(
    app: &tauri::AppHandle,
    payload: &NativeClipboardWritePayload,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let adjacent_office_exe = std::env::current_exe()
        .map_err(|error| format!("Failed to resolve desktop executable path: {error}"))?
        .with_file_name("chemsema-office.exe");
    let mut candidates = vec![adjacent_office_exe];
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("chemsema-office.exe"));
    }
    let office_exe = candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            "ChemSema Office/OLE server was not found next to the desktop executable or in bundled resources."
                .to_string()
        })?;
    if !office_exe.exists() {
        return Err(format!(
            "ChemSema Office/OLE server was not found at {}",
            office_exe.display()
        ));
    }

    let payload_path = std::env::temp_dir().join(format!(
        "chemsema-office-clipboard-{}-{}.json",
        std::process::id(),
        current_timestamp_ms()
    ));
    let json = serde_json::to_string(payload)
        .map_err(|error| format!("Failed to serialize OLE clipboard payload: {error}"))?;
    fs::write(&payload_path, json)
        .map_err(|error| format!("Failed to write OLE clipboard payload: {error}"))?;

    let mut command = std::process::Command::new(&office_exe);
    // The OLE bridge is a short-lived sibling process. Keeping COM clipboard
    // ownership there avoids tying the Tauri window process to Office lifetime.
    command
        .arg("--copy-clipboard-payload")
        .arg(&payload_path)
        .creation_flags(CREATE_NO_WINDOW_FLAG);
    let result = command
        .status()
        .map_err(|error| format!("Failed to run ChemSema Office/OLE clipboard bridge: {error}"))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "ChemSema Office/OLE clipboard bridge exited with {status}."
                ))
            }
        });
    let _ = fs::remove_file(payload_path);
    result
}

#[cfg(not(target_os = "windows"))]
fn native_clipboard_write(
    _app: &tauri::AppHandle,
    _payload: NativeClipboardWritePayload,
) -> Result<(), String> {
    Err("Native clipboard is only implemented on Windows.".to_string())
}

#[cfg(target_os = "windows")]
fn native_clipboard_read() -> Result<NativeClipboardReadPayload, String> {
    use windows_sys::Win32::System::DataExchange::{
        GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return Err("Failed to open Windows clipboard.".to_string());
        }
    }
    let _guard = ClipboardCloseGuard;

    let read_registered = |format_name: &str| -> Result<Option<String>, String> {
        let format = unsafe { RegisterClipboardFormatW(wide_null(format_name).as_ptr()) };
        if format == 0 {
            return Ok(None);
        }
        unsafe {
            if IsClipboardFormatAvailable(format) == 0 {
                return Ok(None);
            }
            let handle = GetClipboardData(format);
            read_clipboard_utf8(handle, GlobalLock, GlobalSize, GlobalUnlock)
        }
    };

    let text = unsafe {
        if IsClipboardFormatAvailable(13) == 0 {
            None
        } else {
            let handle = GetClipboardData(13);
            read_clipboard_utf16(handle, GlobalLock, GlobalSize, GlobalUnlock)?
        }
    };

    Ok(NativeClipboardReadPayload {
        chemsema_fragment_json: read_registered(FORMAT_CHEMSEMA_FRAGMENT)?,
        chemsema_document_json: read_registered(FORMAT_CHEMSEMA_DOCUMENT_JSON)?,
        cdxml: read_registered(FORMAT_CHEMDRAW_INTERCHANGE)?
            .or(read_registered(FORMAT_CDXML_MIME)?),
        svg: read_registered(FORMAT_SVG_MIME)?.or(read_registered(FORMAT_SVG)?),
        text,
    })
}

#[cfg(not(target_os = "windows"))]
fn native_clipboard_read() -> Result<NativeClipboardReadPayload, String> {
    Err("Native clipboard is only implemented on Windows.".to_string())
}

#[cfg(target_os = "windows")]
struct ClipboardCloseGuard;

#[cfg(target_os = "windows")]
impl Drop for ClipboardCloseGuard {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::DataExchange::CloseClipboard();
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
unsafe fn write_clipboard_utf8(
    format: u32,
    value: &str,
    global_alloc: unsafe extern "system" fn(u32, usize) -> *mut std::ffi::c_void,
    global_lock: unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    global_unlock: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
    set_clipboard_data: unsafe extern "system" fn(
        u32,
        *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void,
) -> Result<(), String> {
    let bytes = value.as_bytes();
    let handle = global_alloc(GMEM_MOVEABLE_FLAG, bytes.len() + 1);
    if handle.is_null() {
        return Err("Failed to allocate clipboard memory.".to_string());
    }
    let target = global_lock(handle) as *mut u8;
    if target.is_null() {
        return Err("Failed to lock clipboard memory.".to_string());
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), target, bytes.len());
    *target.add(bytes.len()) = 0;
    global_unlock(handle);
    if set_clipboard_data(format, handle).is_null() {
        return Err("Failed to set clipboard data.".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn write_clipboard_utf16(
    format: u32,
    value: &str,
    global_alloc: unsafe extern "system" fn(u32, usize) -> *mut std::ffi::c_void,
    global_lock: unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    global_unlock: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
    set_clipboard_data: unsafe extern "system" fn(
        u32,
        *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void,
) -> Result<(), String> {
    let wide = wide_null(value);
    let byte_len = wide.len() * std::mem::size_of::<u16>();
    let handle = global_alloc(GMEM_MOVEABLE_FLAG, byte_len);
    if handle.is_null() {
        return Err("Failed to allocate clipboard memory.".to_string());
    }
    let target = global_lock(handle) as *mut u16;
    if target.is_null() {
        return Err("Failed to lock clipboard memory.".to_string());
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr(), target, wide.len());
    global_unlock(handle);
    if set_clipboard_data(format, handle).is_null() {
        return Err("Failed to set clipboard data.".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn read_clipboard_utf8(
    handle: *mut std::ffi::c_void,
    global_lock: unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    global_size: unsafe extern "system" fn(*mut std::ffi::c_void) -> usize,
    global_unlock: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
) -> Result<Option<String>, String> {
    if handle.is_null() {
        return Ok(None);
    }
    let size = global_size(handle);
    if size == 0 {
        return Ok(None);
    }
    let source = global_lock(handle) as *const u8;
    if source.is_null() {
        return Ok(None);
    }
    let slice = std::slice::from_raw_parts(source, size);
    let len = slice
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(slice.len());
    let text = String::from_utf8_lossy(&slice[..len]).to_string();
    global_unlock(handle);
    Ok(Some(text))
}

#[cfg(target_os = "windows")]
unsafe fn read_clipboard_utf16(
    handle: *mut std::ffi::c_void,
    global_lock: unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    global_size: unsafe extern "system" fn(*mut std::ffi::c_void) -> usize,
    global_unlock: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
) -> Result<Option<String>, String> {
    if handle.is_null() {
        return Ok(None);
    }
    let size = global_size(handle);
    if size < std::mem::size_of::<u16>() {
        return Ok(None);
    }
    let source = global_lock(handle) as *const u16;
    if source.is_null() {
        return Ok(None);
    }
    let slice = std::slice::from_raw_parts(source, size / std::mem::size_of::<u16>());
    let len = slice
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(slice.len());
    let text = String::from_utf16_lossy(&slice[..len]);
    global_unlock(handle);
    Ok(Some(text))
}
