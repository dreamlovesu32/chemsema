use crate::*;

#[tauri::command]
pub(crate) fn desktop_clipboard_write(
    app: tauri::AppHandle,
    payload: NativeClipboardWritePayload,
) -> Result<(), String> {
    native_clipboard_write(&app, payload)
}

#[tauri::command]
pub(crate) fn desktop_clipboard_read(
    app: tauri::AppHandle,
) -> Result<NativeClipboardReadPayload, String> {
    native_clipboard_read(&app)
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
fn native_clipboard_read(app: &tauri::AppHandle) -> Result<NativeClipboardReadPayload, String> {
    use windows_sys::Win32::System::DataExchange::{
        GetClipboardData, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    let ole_payload = native_clipboard_needs_ole_bridge()
        .then(|| native_office_ole_clipboard_read(app))
        .flatten();

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

    let clipboard_image = unsafe {
        let png_format = RegisterClipboardFormatW(wide_null("PNG").as_ptr());
        if png_format != 0 && IsClipboardFormatAvailable(png_format) != 0 {
            let bytes = read_clipboard_bytes(
                GetClipboardData(png_format),
                GlobalLock,
                GlobalSize,
                GlobalUnlock,
            )?;
            png_dimensions(&bytes).map(|(width, height)| ("image/png", bytes, width, height))
        } else if IsClipboardFormatAvailable(8) != 0 {
            let dib =
                read_clipboard_bytes(GetClipboardData(8), GlobalLock, GlobalSize, GlobalUnlock)?;
            dib_to_bmp(&dib).map(|(bytes, width, height)| ("image/bmp", bytes, width, height))
        } else {
            None
        }
    };
    let (image_mime_type, image_data_base64, image_pixel_width, image_pixel_height) =
        if let Some((mime_type, bytes, width, height)) = clipboard_image {
            if bytes.len() <= 64 * 1024 * 1024 {
                (
                    Some(mime_type.to_string()),
                    Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
                    Some(width),
                    Some(height),
                )
            } else {
                (None, None, None, None)
            }
        } else {
            (None, None, None, None)
        };

    let html = read_registered(FORMAT_HTML)?;
    let html_payload = html
        .as_deref()
        .and_then(chemsema_payload_from_clipboard_html);
    let html_fragment = html
        .as_deref()
        .and_then(chemsema_fragment_from_clipboard_html);
    let mut payload = NativeClipboardReadPayload {
        chemsema_fragment_json: read_registered(FORMAT_CHEMSEMA_FRAGMENT)?.or(html_fragment),
        chemsema_document_json: read_registered(FORMAT_CHEMSEMA_DOCUMENT_JSON)?,
        cdxml: read_registered(FORMAT_CHEMDRAW_INTERCHANGE)
            .ok()
            .flatten()
            .or(read_registered(FORMAT_CDXML_MIME)?),
        svg: read_registered(FORMAT_SVG_MIME)?.or(read_registered(FORMAT_SVG)?),
        text,
        image_mime_type,
        image_data_base64,
        image_pixel_width,
        image_pixel_height,
    };
    if let Some(html) = html_payload {
        payload.chemsema_fragment_json = payload
            .chemsema_fragment_json
            .or(html.chemsema_fragment_json);
        payload.chemsema_document_json = payload
            .chemsema_document_json
            .or(html.chemsema_document_json);
        payload.cdxml = payload.cdxml.or(html.cdxml);
        payload.svg = payload.svg.or(html.svg);
        payload.text = payload.text.or(html.text);
    }
    if let Some(ole) = ole_payload {
        payload.chemsema_fragment_json = payload
            .chemsema_fragment_json
            .or(ole.chemsema_fragment_json);
        payload.chemsema_document_json = payload
            .chemsema_document_json
            .or(ole.chemsema_document_json);
        payload.cdxml = payload.cdxml.or(ole.cdxml);
        payload.svg = payload.svg.or(ole.svg);
        payload.text = payload.text.or(ole.text);
    }
    Ok(payload)
}

#[cfg(target_os = "windows")]
fn chemsema_fragment_from_clipboard_html(html: &str) -> Option<String> {
    const MARKER: &str = "data-chemsema-clipboard-base64=";
    let tail = html.split_once(MARKER)?.1.trim_start();
    let quote = tail.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let encoded = tail[quote.len_utf8()..].split_once(quote)?.0;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    String::from_utf8(bytes)
        .ok()
        .filter(|value| !value.is_empty())
}

#[cfg(target_os = "windows")]
fn chemsema_payload_from_clipboard_html(html: &str) -> Option<NativeClipboardReadPayload> {
    const MARKER: &str = "data-chemsema-payload-base64=";
    let tail = html.split_once(MARKER)?.1.trim_start();
    let quote = tail.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let encoded = tail[quote.len_utf8()..].split_once(quote)?.0;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(target_os = "windows")]
fn native_clipboard_needs_ole_bridge() -> bool {
    use windows_sys::Win32::System::DataExchange::{
        IsClipboardFormatAvailable, RegisterClipboardFormatW,
    };
    let available = |name: &str| unsafe {
        let format = RegisterClipboardFormatW(wide_null(name).as_ptr());
        format != 0 && IsClipboardFormatAvailable(format) != 0
    };
    let has_direct_structure = [
        FORMAT_CHEMSEMA_FRAGMENT,
        FORMAT_CHEMSEMA_DOCUMENT_JSON,
        FORMAT_CDXML_MIME,
        FORMAT_HTML,
    ]
    .into_iter()
    .any(available);
    !has_direct_structure
        && [
            FORMAT_CHEMDRAW_INTERCHANGE,
            FORMAT_EMBEDDED_OBJECT,
            FORMAT_EMBED_SOURCE,
        ]
        .into_iter()
        .any(available)
}

#[cfg(not(target_os = "windows"))]
fn native_clipboard_read(_app: &tauri::AppHandle) -> Result<NativeClipboardReadPayload, String> {
    Err("Native clipboard is only implemented on Windows.".to_string())
}

#[cfg(target_os = "windows")]
fn native_office_ole_clipboard_read(app: &tauri::AppHandle) -> Option<NativeClipboardReadPayload> {
    use std::os::windows::process::CommandExt;

    let adjacent = std::env::current_exe()
        .ok()?
        .with_file_name("chemsema-office.exe");
    let mut candidates = vec![adjacent];
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("chemsema-office.exe"));
    }
    let office_exe = candidates.into_iter().find(|path| path.exists())?;
    let output_path = std::env::temp_dir().join(format!(
        "chemsema-office-clipboard-read-{}-{}.json",
        std::process::id(),
        current_timestamp_ms()
    ));
    let status = std::process::Command::new(office_exe)
        .arg("--read-clipboard-payload")
        .arg(&output_path)
        .creation_flags(CREATE_NO_WINDOW_FLAG)
        .status()
        .ok()?;
    if !status.success() {
        let _ = fs::remove_file(output_path);
        return None;
    }
    let result = fs::read_to_string(&output_path)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok());
    let _ = fs::remove_file(output_path);
    result
}

#[cfg(target_os = "windows")]
unsafe fn read_clipboard_bytes(
    handle: *mut std::ffi::c_void,
    global_lock: unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void,
    global_size: unsafe extern "system" fn(*mut std::ffi::c_void) -> usize,
    global_unlock: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
) -> Result<Vec<u8>, String> {
    if handle.is_null() {
        return Err("Clipboard image handle is empty.".to_string());
    }
    let size = global_size(handle);
    if size == 0 || size > 64 * 1024 * 1024 {
        return Err("Clipboard image is empty or exceeds 64 MiB.".to_string());
    }
    let source = global_lock(handle) as *const u8;
    if source.is_null() {
        return Err("Failed to lock clipboard image.".to_string());
    }
    let bytes = std::slice::from_raw_parts(source, size).to_vec();
    global_unlock(handle);
    Ok(bytes)
}

#[cfg(target_os = "windows")]
fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..8) != Some(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    Some((
        u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?),
        u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?),
    ))
}

#[cfg(target_os = "windows")]
fn dib_to_bmp(dib: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let header_size = u32::from_le_bytes(dib.get(0..4)?.try_into().ok()?) as usize;
    if header_size < 40 || header_size > dib.len() {
        return None;
    }
    let width = i32::from_le_bytes(dib.get(4..8)?.try_into().ok()?).unsigned_abs();
    let height = i32::from_le_bytes(dib.get(8..12)?.try_into().ok()?).unsigned_abs();
    let bit_count = u16::from_le_bytes(dib.get(14..16)?.try_into().ok()?);
    let compression = u32::from_le_bytes(dib.get(16..20)?.try_into().ok()?);
    let colors_used = u32::from_le_bytes(dib.get(32..36)?.try_into().ok()?) as usize;
    if width == 0 || height == 0 || width > 32_768 || height > 32_768 {
        return None;
    }
    let palette_entries = if colors_used > 0 {
        colors_used
    } else if bit_count <= 8 {
        1usize.checked_shl(u32::from(bit_count))?
    } else {
        0
    };
    let external_masks = if header_size == 40 && compression == 3 {
        12
    } else {
        0
    };
    let pixel_offset = 14usize
        .checked_add(header_size)?
        .checked_add(external_masks)?
        .checked_add(palette_entries.checked_mul(4)?)?;
    if pixel_offset > 14 + dib.len() {
        return None;
    }
    let file_size = 14usize.checked_add(dib.len())?;
    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&(pixel_offset as u32).to_le_bytes());
    bmp.extend_from_slice(dib);
    Some((bmp, width, height))
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

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn portable_html_payload_recovers_all_structured_fallbacks() {
        let json = serde_json::json!({
            "chemsemaFragmentJson": "{\"nodes\":[],\"bonds\":[]}",
            "chemsemaDocumentJson": "{\"format\":{\"name\":\"chemsema\",\"version\":\"0.1\"}}",
            "cdxml": "<CDXML></CDXML>",
            "text": "<CDXML></CDXML>"
        })
        .to_string();
        let encoded = base64::engine::general_purpose::STANDARD.encode(json.as_bytes());
        let html = format!(
            "Version:0.9\r\n<html><body><div data-chemsema-payload-base64=\"{encoded}\"></div></body></html>"
        );
        let payload =
            chemsema_payload_from_clipboard_html(&html).expect("portable payload should decode");
        assert_eq!(
            payload.chemsema_fragment_json.as_deref(),
            Some("{\"nodes\":[],\"bonds\":[]}")
        );
        assert_eq!(payload.cdxml.as_deref(), Some("<CDXML></CDXML>"));
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
