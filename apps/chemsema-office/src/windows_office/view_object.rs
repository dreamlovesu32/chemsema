use super::*;

pub(super) unsafe extern "system" fn view_object_draw(
    this: *mut c_void,
    draw_aspect: u32,
    _index: i32,
    _aspect: *mut c_void,
    _target_device: *mut c_void,
    _target_dc: isize,
    _draw_dc: isize,
    bounds: *const c_void,
    _window_bounds: *const c_void,
    _continue_fn: *mut c_void,
    _continue_value: usize,
) -> i32 {
    if draw_aspect != DVASPECT_CONTENT {
        return DV_E_FORMATETC;
    }
    if _draw_dc == 0 || bounds.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<ViewObject2Vtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    if !draw_payload_preview(
        _draw_dc as HDC,
        &*(bounds.cast::<RECT>()),
        &(*object).payload,
    ) {
        draw_placeholder_preview(_draw_dc as HDC, &*(bounds.cast::<RECT>()));
    }
    S_OK
}

pub(super) unsafe extern "system" fn view_object_get_color_set(
    _this: *mut c_void,
    _draw_aspect: u32,
    _index: i32,
    _aspect: *mut c_void,
    _target_device: *mut c_void,
    _target_dc: isize,
    color_set: *mut *mut c_void,
) -> i32 {
    if !color_set.is_null() {
        *color_set = null_mut();
    }
    E_NOTIMPL
}

pub(super) unsafe extern "system" fn view_object_freeze(
    _this: *mut c_void,
    _draw_aspect: u32,
    _index: i32,
    _aspect: *mut c_void,
    freeze_key: *mut u32,
) -> i32 {
    if !freeze_key.is_null() {
        *freeze_key = 0;
    }
    E_NOTIMPL
}

pub(super) unsafe extern "system" fn view_object_unfreeze(
    _this: *mut c_void,
    _freeze_key: u32,
) -> i32 {
    E_NOTIMPL
}

pub(super) unsafe extern "system" fn view_object_set_advise(
    this: *mut c_void,
    aspects: u32,
    advf: u32,
    sink: *mut c_void,
) -> i32 {
    let object = owner_from_part::<ViewObject2Vtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    com_release((*object).view_advise_sink);
    (*object).view_advise_sink = null_mut();
    (*object).view_advise_aspects = aspects;
    (*object).view_advise_flags = advf;
    if !sink.is_null() {
        com_add_ref(sink);
        (*object).view_advise_sink = sink;
    }
    log_ole_event(&format!(
        "IViewObject2::SetAdvise(aspects=0x{aspects:X}, advf=0x{advf:X}, sink={})",
        if sink.is_null() { "null" } else { "set" }
    ));
    S_OK
}

pub(super) unsafe extern "system" fn view_object_get_advise(
    this: *mut c_void,
    aspects: *mut u32,
    advf: *mut u32,
    sink: *mut *mut c_void,
) -> i32 {
    let object = owner_from_part::<ViewObject2Vtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    if !aspects.is_null() {
        *aspects = (*object).view_advise_aspects;
    }
    if !advf.is_null() {
        *advf = (*object).view_advise_flags;
    }
    if !sink.is_null() {
        *sink = (*object).view_advise_sink;
        com_add_ref(*sink);
    }
    S_OK
}

pub(super) unsafe extern "system" fn view_object_get_extent(
    this: *mut c_void,
    _draw_aspect: u32,
    _index: i32,
    _target_device: *mut c_void,
    size: *mut c_void,
) -> i32 {
    if size.is_null() {
        return E_POINTER;
    }
    let object = owner_from_part::<ViewObject2Vtbl>(this);
    if object.is_null() {
        return E_POINTER;
    }
    *(size.cast::<SIZE>()) = (*object).extent_himetric;
    S_OK
}

pub(super) unsafe extern "system" fn runnable_object_get_running_class(
    _this: *mut c_void,
    class_id: *mut GUID,
) -> i32 {
    if class_id.is_null() {
        return E_POINTER;
    }
    *class_id = CLSID_CHEMSEMA_DOCUMENT;
    S_OK
}

pub(super) unsafe extern "system" fn runnable_object_run(
    _this: *mut c_void,
    _bind_context: *mut c_void,
) -> i32 {
    S_OK
}

pub(super) unsafe extern "system" fn runnable_object_is_running(_this: *mut c_void) -> i32 {
    S_OK
}

pub(super) unsafe extern "system" fn runnable_object_lock_running(
    _this: *mut c_void,
    _lock: i32,
    _last_unlock_closes: i32,
) -> i32 {
    S_OK
}

pub(super) unsafe extern "system" fn runnable_object_set_contained_object(
    _this: *mut c_void,
    _contained: i32,
) -> i32 {
    S_OK
}

pub(super) unsafe fn com_add_ref(interface: *mut c_void) {
    if interface.is_null() {
        return;
    }
    let vtbl = *(interface.cast::<*const UnknownVtbl>());
    ((*vtbl).add_ref)(interface);
}

pub(super) unsafe fn com_release(interface: *mut c_void) {
    if interface.is_null() {
        return;
    }
    let vtbl = *(interface.cast::<*const UnknownVtbl>());
    ((*vtbl).release)(interface);
}
