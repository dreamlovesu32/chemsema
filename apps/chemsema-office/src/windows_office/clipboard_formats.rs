use super::*;

pub(super) fn clipboard_format(name: &str) -> u16 {
    unsafe { RegisterClipboardFormatW(wide_null(name).as_ptr()) as u16 }
}

pub(super) fn known_clipboard_format_name(format: u16) -> &'static str {
    if format == clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE) {
        CLIPBOARD_FORMAT_EMBED_SOURCE
    } else if format == clipboard_format(CLIPBOARD_FORMAT_EMBEDDED_OBJECT) {
        CLIPBOARD_FORMAT_EMBEDDED_OBJECT
    } else if format == clipboard_format(CLIPBOARD_FORMAT_NATIVE) {
        CLIPBOARD_FORMAT_NATIVE
    } else if format == clipboard_format(CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR) {
        CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR
    } else if format == clipboard_format(CLIPBOARD_FORMAT_RTF) {
        CLIPBOARD_FORMAT_RTF
    } else if format == clipboard_format(CLIPBOARD_FORMAT_HTML) {
        CLIPBOARD_FORMAT_HTML
    } else if format == clipboard_format(FORMAT_CHEMSEMA_NATIVE) {
        FORMAT_CHEMSEMA_NATIVE
    } else if format == CF_ENHMETAFILE {
        "CF_ENHMETAFILE"
    } else if format == CF_METAFILEPICT {
        "CF_METAFILEPICT"
    } else if format == clipboard_format(FORMAT_CHEMSEMA_FRAGMENT) {
        FORMAT_CHEMSEMA_FRAGMENT
    } else if format == clipboard_format(FORMAT_CHEMSEMA_DOCUMENT_JSON) {
        FORMAT_CHEMSEMA_DOCUMENT_JSON
    } else if format == clipboard_format(FORMAT_CHEMDRAW_INTERCHANGE) {
        FORMAT_CHEMDRAW_INTERCHANGE
    } else if format == clipboard_format(FORMAT_CDXML_MIME) {
        FORMAT_CDXML_MIME
    } else if format == clipboard_format(FORMAT_SVG_MIME) {
        FORMAT_SVG_MIME
    } else if format == clipboard_format(FORMAT_SVG) {
        FORMAT_SVG
    } else if format == CF_UNICODETEXT_FORMAT {
        "CF_UNICODETEXT"
    } else {
        "unknown"
    }
}

pub(super) unsafe fn log_format_request(prefix: &str, format: &FORMATETC, hr: i32) {
    log_ole_event(&format!(
        "{prefix}: cf={} ({}) aspect={} lindex={} tymed=0x{:X} -> 0x{:08X}",
        format.cfFormat,
        known_clipboard_format_name(format.cfFormat),
        format.dwAspect,
        format.lindex,
        format.tymed,
        hr as u32
    ));
}

pub(super) fn default_misc_status() -> u32 {
    (OLEMISC_RENDERINGISDEVICEINDEPENDENT | OLEMISC_SETCLIENTSITEFIRST) as u32
}

pub(super) fn ole_clipboard_formats(payload: &OleObjectPayload, _extent: SIZE) -> Vec<FORMATETC> {
    let mut formats = Vec::new();
    push_format(
        &mut formats,
        clipboard_format(CLIPBOARD_FORMAT_EMBEDDED_OBJECT),
        TYMED_ISTORAGE as u32,
    );
    push_format(
        &mut formats,
        clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE),
        TYMED_ISTORAGE as u32,
    );
    push_format(
        &mut formats,
        clipboard_format(CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR),
        TYMED_HGLOBAL as u32,
    );
    push_format(
        &mut formats,
        clipboard_format(CLIPBOARD_FORMAT_NATIVE),
        TYMED_HGLOBAL as u32,
    );
    if payload.chemsema_fragment_json.is_some() {
        push_format(
            &mut formats,
            clipboard_format(FORMAT_CHEMSEMA_FRAGMENT),
            TYMED_HGLOBAL as u32,
        );
    }
    push_format(
        &mut formats,
        clipboard_format(FORMAT_CHEMSEMA_NATIVE),
        TYMED_HGLOBAL as u32,
    );
    push_format(
        &mut formats,
        clipboard_format(FORMAT_CHEMSEMA_DOCUMENT_JSON),
        TYMED_HGLOBAL as u32,
    );
    if payload.cdxml.is_some() {
        push_format(
            &mut formats,
            clipboard_format(FORMAT_CDXML_MIME),
            TYMED_HGLOBAL as u32,
        );
    }
    if payload.chemsema_fragment_json.is_some() {
        push_format(
            &mut formats,
            clipboard_format(CLIPBOARD_FORMAT_HTML),
            TYMED_HGLOBAL as u32,
        );
    }
    if payload.text.is_some() {
        push_format(&mut formats, CF_UNICODETEXT_FORMAT, TYMED_HGLOBAL as u32);
    }
    push_format(&mut formats, CF_ENHMETAFILE, TYMED_ENHMF as u32);

    formats.retain(|format| format.cfFormat != 0);
    formats
}

pub(super) fn push_format(formats: &mut Vec<FORMATETC>, cf_format: u16, tymed: u32) {
    if cf_format == 0 {
        return;
    }
    formats.push(FORMATETC {
        cfFormat: cf_format,
        ptd: null_mut(),
        dwAspect: DVASPECT_CONTENT,
        lindex: -1,
        tymed,
    });
}

pub(super) unsafe fn clipboard_format_supported(
    payload: &OleObjectPayload,
    extent: SIZE,
    requested: &FORMATETC,
) -> bool {
    ole_clipboard_formats(payload, extent)
        .into_iter()
        .any(|available| {
            available.cfFormat == requested.cfFormat
                && (requested.dwAspect == 0 || requested.dwAspect == DVASPECT_CONTENT)
                && (available.tymed & requested.tymed) != 0
        })
}

pub(super) unsafe fn write_clipboard_format_to_medium(
    payload: &OleObjectPayload,
    extent: SIZE,
    format: &FORMATETC,
    medium: *mut STGMEDIUM,
) -> i32 {
    if medium.is_null() {
        return E_POINTER;
    }
    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_EMBEDDED_OBJECT)
        || format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_EMBED_SOURCE)
    {
        if (format.tymed & TYMED_ISTORAGE as u32) == 0 {
            return DV_E_TYMED;
        }
        return create_ole_storage_medium(payload, extent, medium);
    }
    if format.cfFormat == CF_ENHMETAFILE {
        if (format.tymed & TYMED_ENHMF as u32) == 0 {
            return DV_E_TYMED;
        }
        return match enhanced_metafile_for_office_payload(payload, extent) {
            Ok(handle) => enhanced_metafile_medium(handle, medium),
            Err(hr) => hr,
        };
    }
    if format.cfFormat == CF_METAFILEPICT {
        if (format.tymed & TYMED_MFPICT as u32) == 0 {
            return DV_E_TYMED;
        }
        return match hglobal_for_metafile_pict(payload, extent) {
            Ok(handle) => metafile_pict_medium(handle, medium),
            Err(hr) => hr,
        };
    }
    if (format.tymed & TYMED_HGLOBAL as u32) == 0 {
        return DV_E_TYMED;
    }

    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_OBJECT_DESCRIPTOR) {
        return match hglobal_for_object_descriptor(extent) {
            Ok(handle) => hglobal_medium(handle, medium),
            Err(hr) => hr,
        };
    }
    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_RTF) {
        return match hglobal_for_word_rtf_object(payload) {
            Ok(handle) => hglobal_medium(handle, medium),
            Err(hr) => hr,
        };
    }
    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_HTML) {
        return hglobal_text_medium(&clipboard_html(payload), false, medium);
    }
    if format.cfFormat == clipboard_format(FORMAT_CHEMSEMA_FRAGMENT) {
        return payload
            .chemsema_fragment_json
            .as_deref()
            .map(|value| hglobal_text_medium(value, false, medium))
            .unwrap_or(DV_E_FORMATETC);
    }
    if format.cfFormat == clipboard_format(CLIPBOARD_FORMAT_NATIVE) {
        return match hglobal_for_native_clipboard_payload(payload) {
            Ok(handle) => hglobal_medium(handle, medium),
            Err(hr) => hr,
        };
    }
    if format.cfFormat == clipboard_format(FORMAT_CHEMSEMA_NATIVE) {
        return hglobal_text_medium(&payload.chemsema_document_json, false, medium);
    }
    if format.cfFormat == clipboard_format(FORMAT_CHEMSEMA_DOCUMENT_JSON) {
        return hglobal_text_medium(&payload.chemsema_document_json, false, medium);
    }
    if format.cfFormat == clipboard_format(FORMAT_CDXML_MIME) {
        return payload
            .cdxml
            .as_deref()
            .map(|value| hglobal_text_medium(value, false, medium))
            .unwrap_or(DV_E_FORMATETC);
    }
    if format.cfFormat == clipboard_format(FORMAT_SVG_MIME)
        || format.cfFormat == clipboard_format(FORMAT_SVG)
    {
        return hglobal_text_medium(&payload.svg, false, medium);
    }
    if format.cfFormat == CF_UNICODETEXT_FORMAT {
        return payload
            .text
            .as_deref()
            .map(|value| hglobal_text_medium(value, true, medium))
            .unwrap_or(DV_E_FORMATETC);
    }
    DV_E_FORMATETC
}

pub(super) fn clipboard_html(payload: &OleObjectPayload) -> String {
    let fragment_json = payload.chemsema_fragment_json.as_deref().unwrap_or("");
    let encoded = base64::engine::general_purpose::STANDARD.encode(fragment_json.as_bytes());
    let portable_json =
        serde_json::to_vec(&payload.clipboard_payload()).unwrap_or_else(|_| b"{}".to_vec());
    let payload_encoded = base64::engine::general_purpose::STANDARD.encode(portable_json);
    let fragment = format!(
        "<div data-chemsema-payload-base64=\"{payload_encoded}\" data-chemsema-clipboard-base64=\"{encoded}\"></div>"
    );
    let prefix = "<html><body><!--StartFragment-->";
    let suffix = "<!--EndFragment--></body></html>";
    let header_template = concat!(
        "Version:0.9\r\n",
        "StartHTML:0000000000\r\n",
        "EndHTML:0000000000\r\n",
        "StartFragment:0000000000\r\n",
        "EndFragment:0000000000\r\n"
    );
    let start_html = header_template.len();
    let start_fragment = start_html + prefix.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = end_fragment + suffix.len();
    let header = format!(
        "Version:0.9\r\nStartHTML:{start_html:010}\r\nEndHTML:{end_html:010}\r\nStartFragment:{start_fragment:010}\r\nEndFragment:{end_fragment:010}\r\n"
    );
    format!("{header}{prefix}{fragment}{suffix}")
}
