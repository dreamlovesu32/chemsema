use super::*;

pub(super) unsafe extern "system" fn ole_object_set_client_site(
    this: *mut c_void,
    site: *mut c_void,
) -> i32 {
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    com_release((*object).client_site);
    (*object).client_site = site;
    com_add_ref(site);
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_get_client_site(
    this: *mut c_void,
    site: *mut *mut c_void,
) -> i32 {
    if site.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        *site = null_mut();
        return E_POINTER;
    }
    *site = (*object).client_site;
    com_add_ref(*site);
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_set_host_names(
    _this: *mut c_void,
    _container_app: *const u16,
    _container_object: *const u16,
) -> i32 {
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_close(
    _this: *mut c_void,
    _save_option: i32,
) -> i32 {
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_set_moniker(
    _this: *mut c_void,
    _which_moniker: u32,
    _moniker: *mut c_void,
) -> i32 {
    E_NOTIMPL
}

pub(super) unsafe extern "system" fn ole_object_get_moniker(
    _this: *mut c_void,
    _assign: u32,
    _which_moniker: u32,
    moniker: *mut *mut c_void,
) -> i32 {
    if !moniker.is_null() {
        *moniker = null_mut();
    }
    E_NOTIMPL
}

pub(super) unsafe extern "system" fn ole_object_init_from_data(
    this: *mut c_void,
    data_object: *mut c_void,
    _creation: i32,
    _reserved: u32,
) -> i32 {
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() || data_object.is_null() {
        return E_POINTER;
    }
    match payload_from_data_object(data_object) {
        Ok(payload) => {
            (*object).payload = payload;
            (*object).extent_himetric = (*object).payload.extent_himetric();
            S_OK
        }
        Err(error) => {
            log_ole_event(&format!("IOleObject::InitFromData failed: {error}"));
            E_FAIL
        }
    }
}

pub(super) unsafe extern "system" fn ole_object_get_clipboard_data(
    this: *mut c_void,
    _reserved: u32,
    data_object: *mut *mut c_void,
) -> i32 {
    if data_object.is_null() {
        return E_POINTER;
    }
    *data_object = null_mut();
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    *data_object =
        (&mut (*object).data_object as *mut InterfacePart<DataObjectVtbl>).cast::<c_void>();
    chemsema_object_add_ref(object);
    log_ole_event("IOleObject::GetClipboardData -> IDataObject");
    S_OK
}

pub(super) unsafe fn payload_from_data_object(
    data_object: *mut c_void,
) -> Result<OleObjectPayload, String> {
    if data_object.is_null() {
        return Err("IDataObject pointer was null.".to_string());
    }
    let get_data = (*(*(data_object.cast::<*const DataObjectVtbl>()))).get_data;

    for format_name in [
        CLIPBOARD_FORMAT_EMBEDDED_OBJECT,
        CLIPBOARD_FORMAT_EMBED_SOURCE,
    ] {
        let format = FORMATETC {
            cfFormat: clipboard_format(format_name),
            ptd: null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_ISTORAGE as u32,
        };
        let mut medium = STGMEDIUM::default();
        let hr = get_data(data_object, &format, &mut medium);
        if !hresult_succeeded(hr) {
            continue;
        }
        let result = if medium.tymed == TYMED_ISTORAGE as u32 && !medium.u.pstg.is_null() {
            payload_from_storage(medium.u.pstg)
        } else {
            Err(format!(
                "IDataObject::GetData({format_name}) returned unexpected medium."
            ))
        };
        ReleaseStgMedium(&mut medium);
        if result.is_ok() {
            return result;
        }
    }

    let mut payload = OleObjectPayload::blank();
    let mut populated = false;

    if let Some(document) =
        text_payload_from_data_object(data_object, FORMAT_CHEMSEMA_NATIVE, false)?.or(
            text_payload_from_data_object(data_object, FORMAT_CHEMSEMA_DOCUMENT_JSON, false)?,
        )
    {
        payload.chemsema_document_json = document;
        payload.document_was_supplied = true;
        populated = true;
    }
    if let Some(fragment) =
        text_payload_from_data_object(data_object, FORMAT_CHEMSEMA_FRAGMENT, false)?
    {
        payload.chemsema_fragment_json = Some(fragment);
        populated = true;
    }
    if let Some(cdxml) = chemical_payload_from_data_object(data_object)? {
        if payload.text.is_none() {
            payload.text = Some(cdxml.clone());
        }
        payload.cdxml = Some(cdxml);
        populated = true;
    }
    if let Some(svg) = text_payload_from_data_object(data_object, FORMAT_SVG_MIME, false)?.or(
        text_payload_from_data_object(data_object, FORMAT_SVG, false)?,
    ) {
        payload.svg = svg;
        payload.svg_was_supplied = true;
        populated = true;
    }
    if let Some(text) =
        text_payload_from_data_object_by_id(data_object, CF_UNICODETEXT_FORMAT, true)?
    {
        payload.text = Some(text);
        populated = true;
    }

    if populated {
        Ok(payload)
    } else {
        Err("ChemSema payload was not available from IDataObject.".to_string())
    }
}

pub(super) unsafe fn chemical_payload_from_data_object(
    data_object: *mut c_void,
) -> Result<Option<String>, String> {
    for format_name in [FORMAT_CHEMDRAW_INTERCHANGE, FORMAT_CDXML_MIME] {
        let Some(bytes) =
            bytes_payload_from_data_object_by_id(data_object, clipboard_format(format_name))?
        else {
            continue;
        };
        if bytes.starts_with(b"VjCD0100") {
            return chemsema_engine::cdx_to_cdxml(&bytes).map(Some);
        }
        let end = bytes
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(bytes.len());
        if let Ok(text) = String::from_utf8(bytes[..end].to_vec()) {
            if text.contains("<CDXML") {
                return Ok(Some(text));
            }
        }
    }
    Ok(None)
}

pub(super) unsafe fn bytes_payload_from_data_object_by_id(
    data_object: *mut c_void,
    cf_format: u16,
) -> Result<Option<Vec<u8>>, String> {
    if data_object.is_null() || cf_format == 0 {
        return Ok(None);
    }
    let get_data = (*(*(data_object.cast::<*const DataObjectVtbl>()))).get_data;
    let format = FORMATETC {
        cfFormat: cf_format,
        ptd: null_mut(),
        dwAspect: DVASPECT_CONTENT,
        lindex: -1,
        tymed: TYMED_HGLOBAL as u32,
    };
    let mut medium = STGMEDIUM::default();
    let hr = get_data(data_object, &format, &mut medium);
    if !hresult_succeeded(hr) {
        return Ok(None);
    }
    let result = if medium.tymed == TYMED_HGLOBAL as u32 && !medium.u.hGlobal.is_null() {
        let size = GlobalSize(medium.u.hGlobal);
        let source = GlobalLock(medium.u.hGlobal);
        if size == 0 || source.is_null() {
            Ok(None)
        } else {
            let bytes = std::slice::from_raw_parts(source.cast::<u8>(), size).to_vec();
            GlobalUnlock(medium.u.hGlobal);
            Ok(Some(bytes))
        }
    } else {
        Ok(None)
    };
    ReleaseStgMedium(&mut medium);
    result
}

pub(super) unsafe fn payload_from_storage(
    storage: *mut c_void,
) -> Result<OleObjectPayload, String> {
    if storage.is_null() {
        return Err("IStorage pointer was null.".to_string());
    }
    let mut payload = OleObjectPayload::blank();
    let mut populated = false;

    if let Ok(document) = storage_read_stream(storage, OLE_STREAM_DOCUMENT).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemSemaDocument stream is not UTF-8: {error}"))
    }) {
        payload.chemsema_document_json = document;
        populated = true;
    } else if let Ok(contents) = storage_read_stream(storage, OLE_STREAM_CONTENTS) {
        if contents.starts_with(b"VjCD0100") {
            if let Ok(cdxml) = chemsema_engine::cdx_to_cdxml(&contents) {
                payload.text = Some(cdxml.clone());
                payload.cdxml = Some(cdxml);
                populated = true;
            }
        } else if let Ok(text) = String::from_utf8(contents) {
            if text.contains("<CDXML") {
                payload.text = Some(text.clone());
                payload.cdxml = Some(text);
                populated = true;
            } else if chemsema_engine::parse_document_json(&text).is_ok() {
                payload.chemsema_document_json = text;
                payload.document_was_supplied = true;
                populated = true;
            }
        }
    }
    if let Ok(svg) = storage_read_stream(storage, OLE_STREAM_PREVIEW_SVG).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemSemaPreviewSvg stream is not UTF-8: {error}"))
    }) {
        payload.svg = svg;
        payload.svg_was_supplied = true;
        populated = true;
    }
    if let Ok(cdxml) = storage_read_stream(storage, OLE_STREAM_SOURCE_CDXML).and_then(|bytes| {
        String::from_utf8(bytes)
            .map_err(|error| format!("ChemSemaSourceCdxml stream is not UTF-8: {error}"))
    }) {
        payload.text = Some(cdxml.clone());
        payload.cdxml = Some(cdxml);
        populated = true;
    }

    if populated {
        Ok(payload)
    } else {
        Err("ChemSema payload storage did not contain any readable streams.".to_string())
    }
}

pub(super) unsafe fn text_payload_from_data_object(
    data_object: *mut c_void,
    format_name: &str,
    unicode: bool,
) -> Result<Option<String>, String> {
    text_payload_from_data_object_by_id(data_object, clipboard_format(format_name), unicode)
}

pub(super) unsafe fn text_payload_from_data_object_by_id(
    data_object: *mut c_void,
    cf_format: u16,
    unicode: bool,
) -> Result<Option<String>, String> {
    if data_object.is_null() || cf_format == 0 {
        return Ok(None);
    }
    let get_data = (*(*(data_object.cast::<*const DataObjectVtbl>()))).get_data;
    let format = FORMATETC {
        cfFormat: cf_format,
        ptd: null_mut(),
        dwAspect: DVASPECT_CONTENT,
        lindex: -1,
        tymed: TYMED_HGLOBAL as u32,
    };
    let mut medium = STGMEDIUM::default();
    let hr = get_data(data_object, &format, &mut medium);
    if !hresult_succeeded(hr) {
        return Ok(None);
    }
    let result = if medium.tymed == TYMED_HGLOBAL as u32 && !medium.u.hGlobal.is_null() {
        read_hglobal_text(medium.u.hGlobal, unicode)
    } else {
        Ok(None)
    };
    ReleaseStgMedium(&mut medium);
    result
}

pub(super) unsafe fn read_hglobal_text(
    handle: HGLOBAL,
    unicode: bool,
) -> Result<Option<String>, String> {
    if handle.is_null() {
        return Ok(None);
    }
    let size = GlobalSize(handle);
    if size == 0 {
        return Ok(None);
    }
    let source = GlobalLock(handle);
    if source.is_null() {
        return Err("Failed to lock HGLOBAL text payload.".to_string());
    }
    let result = if unicode {
        let len = size / std::mem::size_of::<u16>();
        let wide = std::slice::from_raw_parts(source.cast::<u16>(), len);
        let end = wide
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(wide.len());
        String::from_utf16(&wide[..end])
            .map(Some)
            .map_err(|error| format!("Failed to decode UTF-16 HGLOBAL payload: {error}"))
    } else {
        let bytes = std::slice::from_raw_parts(source.cast::<u8>(), size);
        let end = bytes
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(bytes.len());
        String::from_utf8(bytes[..end].to_vec())
            .map(Some)
            .map_err(|error| format!("Failed to decode UTF-8 HGLOBAL payload: {error}"))
    };
    GlobalUnlock(handle);
    result
}

pub(super) unsafe extern "system" fn ole_object_do_verb(
    this: *mut c_void,
    verb: i32,
    _message: *mut c_void,
    _active_site: *mut c_void,
    _index: i32,
    _parent: isize,
    _position: *const c_void,
) -> i32 {
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    log_ole_event(&format!("IOleObject::DoVerb({verb})"));
    match launch_desktop_for_object(object) {
        Ok(()) => {
            log_ole_event(&format!("IOleObject::DoVerb({verb}) -> 0x00000000"));
            S_OK
        }
        Err(error) => {
            log_ole_event(&format!("DoVerb({verb}) failed: {error}"));
            OLE_E_NOTRUNNING
        }
    }
}

pub(super) unsafe extern "system" fn ole_object_enum_verbs(
    _this: *mut c_void,
    enum_verbs: *mut *mut c_void,
) -> i32 {
    if enum_verbs.is_null() {
        return E_POINTER;
    }
    *enum_verbs = null_mut();
    OleRegEnumVerbs(&CLSID_CHEMSEMA_DOCUMENT, enum_verbs)
}

pub(super) unsafe extern "system" fn ole_object_update(_this: *mut c_void) -> i32 {
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_is_up_to_date(_this: *mut c_void) -> i32 {
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_get_user_class_id(
    _this: *mut c_void,
    class_id: *mut GUID,
) -> i32 {
    if class_id.is_null() {
        return E_POINTER;
    }
    *class_id = CLSID_CHEMSEMA_DOCUMENT;
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_get_user_type(
    _this: *mut c_void,
    form: u32,
    user_type: *mut *mut u16,
) -> i32 {
    if user_type.is_null() {
        return E_POINTER;
    }
    *user_type = null_mut();
    let hr = OleRegGetUserType(&CLSID_CHEMSEMA_DOCUMENT, form, user_type);
    if hresult_succeeded(hr) {
        return hr;
    }
    allocate_com_string(DOCUMENT_DISPLAY_NAME, user_type)
}

pub(super) unsafe extern "system" fn ole_object_set_extent(
    this: *mut c_void,
    _draw_aspect: u32,
    size: *const c_void,
) -> i32 {
    if size.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    let requested = *(size.cast::<SIZE>());
    let natural = (*object).payload.extent_himetric();
    if requested.cx == DEFAULT_OBJECT_WIDTH_HIMETRIC
        && requested.cy == DEFAULT_OBJECT_HEIGHT_HIMETRIC
        && (natural.cx != requested.cx || natural.cy != requested.cy)
    {
        (*object).extent_himetric = natural;
    } else {
        (*object).extent_himetric = requested;
    }
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_get_extent(
    this: *mut c_void,
    _draw_aspect: u32,
    size: *mut c_void,
) -> i32 {
    if size.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    *(size.cast::<SIZE>()) = (*object).extent_himetric;
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_advise(
    this: *mut c_void,
    sink: *mut c_void,
    connection: *mut u32,
) -> i32 {
    if connection.is_null() {
        return E_POINTER;
    }
    *connection = 0;
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    if (*object).ole_advise_holder.is_null() {
        let hr = CreateOleAdviseHolder(&mut (*object).ole_advise_holder);
        if !hresult_succeeded(hr) {
            return hr;
        }
    }
    let holder_vtbl = *((*object)
        .ole_advise_holder
        .cast::<*const OleAdviseHolderVtbl>());
    ((*holder_vtbl).advise)((*object).ole_advise_holder, sink, connection)
}

pub(super) unsafe extern "system" fn ole_object_unadvise(
    this: *mut c_void,
    connection: u32,
) -> i32 {
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() || (*object).ole_advise_holder.is_null() {
        return E_POINTER;
    }
    let holder_vtbl = *((*object)
        .ole_advise_holder
        .cast::<*const OleAdviseHolderVtbl>());
    ((*holder_vtbl).unadvise)((*object).ole_advise_holder, connection)
}

pub(super) unsafe extern "system" fn ole_object_enum_advise(
    this: *mut c_void,
    enum_advise: *mut *mut c_void,
) -> i32 {
    if enum_advise.is_null() {
        return E_POINTER;
    }
    *enum_advise = null_mut();
    let object = owner_from_part::<OleObjectVtbl>(this);
    if object.is_null() || (*object).ole_advise_holder.is_null() {
        return S_FALSE;
    }
    let holder_vtbl = *((*object)
        .ole_advise_holder
        .cast::<*const OleAdviseHolderVtbl>());
    ((*holder_vtbl).enum_advise)((*object).ole_advise_holder, enum_advise)
}

pub(super) unsafe extern "system" fn ole_object_get_misc_status(
    _this: *mut c_void,
    draw_aspect: u32,
    status: *mut u32,
) -> i32 {
    if status.is_null() {
        return E_POINTER;
    }
    let hr = OleRegGetMiscStatus(&CLSID_CHEMSEMA_DOCUMENT, draw_aspect, status);
    if hresult_succeeded(hr) {
        return hr;
    }
    *status = default_misc_status();
    S_OK
}

pub(super) unsafe extern "system" fn ole_object_set_color_scheme(
    _this: *mut c_void,
    _palette_log: *const c_void,
) -> i32 {
    S_OK
}
