use super::*;

pub(super) fn ole_preview_svg_stream_payload() -> Vec<u8> {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="240" height="120" viewBox="0 0 240 120"><rect width="240" height="120" fill="#ffffff"/><path d="M56 68h128" stroke="#111827" stroke-width="4" stroke-linecap="round"/><circle cx="56" cy="68" r="7" fill="#111827"/><circle cx="184" cy="68" r="7" fill="#111827"/><text x="120" y="32" text-anchor="middle" font-family="Arial, sans-serif" font-size="16" fill="#111827">{DOCUMENT_DISPLAY_NAME}</text></svg>"##
    )
    .into_bytes()
}

pub(super) fn ole_manifest_stream_payload(presentation_extent: SIZE) -> Result<Vec<u8>, i32> {
    serde_json::to_vec(&serde_json::json!({
        "format": "chemsema-ole-object",
        "version": 1,
        "classId": CLSID_STRING,
        "progId": PROG_ID,
        "presentationExtentHimetric": {
            "cx": presentation_extent.cx,
            "cy": presentation_extent.cy
        },
        "documentStream": OLE_STREAM_DOCUMENT,
        "previewSvgStream": OLE_STREAM_PREVIEW_SVG,
        "enhancedPrintStream": OLE_STREAM_ENHANCED_PRINT,
        "presentationStreams": [
            OLE_STREAM_PRESENTATION_EMF
        ]
    }))
    .map_err(|_| E_FAIL)
}

pub(super) unsafe fn storage_presentation_extent(storage: *mut c_void) -> Option<SIZE> {
    storage_read_stream(storage, OLE_STREAM_MANIFEST)
        .ok()
        .and_then(|bytes| presentation_extent_from_manifest(&bytes))
        .or_else(|| {
            storage_read_stream(storage, OLE_STREAM_PRESENTATION_EMF)
                .ok()
                .and_then(|bytes| presentation_extent_from_ole_presentation_stream(&bytes))
        })
        .or_else(|| {
            storage_read_stream(storage, OLE_STREAM_PRESENTATION_EMF_WORD)
                .ok()
                .and_then(|bytes| presentation_extent_from_ole_presentation_stream(&bytes))
        })
        .or_else(|| {
            storage_read_stream(storage, OLE_STREAM_ENHANCED_PRINT)
                .ok()
                .and_then(|bytes| presentation_extent_from_emf_bits(&bytes))
        })
}

pub(super) fn presentation_extent_from_manifest(bytes: &[u8]) -> Option<SIZE> {
    let manifest: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let extent = manifest.get("presentationExtentHimetric")?;
    let cx = extent.get("cx")?.as_i64()?;
    let cy = extent.get("cy")?.as_i64()?;
    valid_presentation_extent(cx, cy)
}

pub(super) fn presentation_extent_from_ole_presentation_stream(bytes: &[u8]) -> Option<SIZE> {
    read_size_at(bytes, 28).or_else(|| {
        let emf_signature = 1u32.to_le_bytes();
        bytes
            .windows(4)
            .enumerate()
            .filter(|(_, window)| *window == emf_signature)
            .find_map(|(emf_offset, _)| presentation_extent_from_emf_bits(&bytes[emf_offset..]))
    })
}

pub(super) fn presentation_extent_from_emf_bits(bytes: &[u8]) -> Option<SIZE> {
    read_size_at(bytes, 32)
}

pub(super) fn read_size_at(bytes: &[u8], offset: usize) -> Option<SIZE> {
    let cx = i32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?);
    let cy = i32::from_le_bytes(bytes.get(offset + 4..offset + 8)?.try_into().ok()?);
    valid_presentation_extent(cx as i64, cy as i64)
}

pub(super) fn valid_presentation_extent(cx: i64, cy: i64) -> Option<SIZE> {
    if (MIN_OBJECT_EXTENT_HIMETRIC as i64..=1_000_000).contains(&cx)
        && (MIN_OBJECT_EXTENT_HIMETRIC as i64..=1_000_000).contains(&cy)
    {
        Some(SIZE {
            cx: cx as i32,
            cy: cy as i32,
        })
    } else {
        None
    }
}

pub(super) unsafe fn storage_write_stream(storage: *mut c_void, name: &str, bytes: &[u8]) -> i32 {
    let mut stream = null_mut();
    let name_w = wide_null(name);
    let storage_vtbl = *(storage.cast::<*const StorageVtbl>());
    let hr = ((*storage_vtbl).create_stream)(
        storage,
        name_w.as_ptr(),
        STGM_CREATE | STGM_READWRITE | STGM_SHARE_EXCLUSIVE,
        0,
        0,
        &mut stream,
    );
    if !hresult_succeeded(hr) || stream.is_null() {
        return hr;
    }

    let hr = stream_write_all(stream, bytes);
    let commit_hr = if hresult_succeeded(hr) {
        stream_commit(stream)
    } else {
        hr
    };
    com_release(stream);
    commit_hr
}

pub(super) unsafe fn storage_read_stream(
    storage: *mut c_void,
    name: &str,
) -> Result<Vec<u8>, String> {
    let mut stream = null_mut();
    let name_w = wide_null(name);
    let storage_vtbl = *(storage.cast::<*const StorageVtbl>());
    let hr = ((*storage_vtbl).open_stream)(
        storage,
        name_w.as_ptr(),
        null_mut(),
        STGM_READ | STGM_SHARE_EXCLUSIVE,
        0,
        &mut stream,
    );
    if !hresult_succeeded(hr) || stream.is_null() {
        return Err(format!(
            "IStorage::OpenStream({name}) failed: 0x{:08X}",
            hr as u32
        ));
    }
    let result = stream_read_all(stream).map_err(|hr| {
        format!(
            "IStream::Read for stream {name} failed: 0x{:08X}",
            hr as u32
        )
    });
    com_release(stream);
    result
}

pub(super) unsafe fn storage_commit(storage: *mut c_void) -> i32 {
    let storage_vtbl = *(storage.cast::<*const StorageVtbl>());
    ((*storage_vtbl).commit)(storage, STGC_DEFAULT)
}

pub(super) unsafe fn stream_write_all(stream: *mut c_void, bytes: &[u8]) -> i32 {
    let stream_vtbl = *(stream.cast::<*const StreamVtbl>());
    let mut offset = 0usize;
    while offset < bytes.len() {
        let remaining = (bytes.len() - offset).min(u32::MAX as usize) as u32;
        let mut written = 0;
        let hr = ((*stream_vtbl).write)(
            stream,
            bytes[offset..].as_ptr().cast::<c_void>(),
            remaining,
            &mut written,
        );
        if !hresult_succeeded(hr) {
            return hr;
        }
        if written == 0 {
            return E_FAIL;
        }
        offset += written as usize;
    }
    S_OK
}

pub(super) unsafe fn stream_read_all(stream: *mut c_void) -> Result<Vec<u8>, i32> {
    let stream_vtbl = *(stream.cast::<*const StreamVtbl>());
    let mut out = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let mut read = 0;
        let hr = ((*stream_vtbl).read)(
            stream,
            buffer.as_mut_ptr().cast::<c_void>(),
            buffer.len() as u32,
            &mut read,
        );
        if !hresult_succeeded(hr) {
            return Err(hr);
        }
        if read == 0 {
            break;
        }
        out.extend_from_slice(&buffer[..read as usize]);
        if read < buffer.len() as u32 {
            break;
        }
    }
    Ok(out)
}

pub(super) unsafe fn stream_commit(stream: *mut c_void) -> i32 {
    let stream_vtbl = *(stream.cast::<*const StreamVtbl>());
    ((*stream_vtbl).commit)(stream, STGC_DEFAULT)
}
