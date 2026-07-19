//! Safe, narrow Rust boundary to official IUPAC InChI 1.07.5.
//!
//! The native library is built from the unmodified MIT-licensed upstream C
//! sources under third_party/inchi-1.07.5. Browser builds use the official
//! upstream WebAssembly artifact through the viewer host instead.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InchiResult {
    pub inchi: String,
    pub inchikey: String,
    pub auxiliary_info: Option<String>,
    pub message: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::InchiResult;
    use std::ffi::{c_char, CStr, CString};
    use std::ptr;

    #[repr(C)]
    struct InchiOutput {
        inchi: *mut c_char,
        aux_info: *mut c_char,
        message: *mut c_char,
        log: *mut c_char,
    }

    unsafe extern "C" {
        fn MakeINCHIFromMolfileText(
            moltext: *const c_char,
            options: *mut c_char,
            result: *mut InchiOutput,
        ) -> i32;
        fn FreeINCHI(result: *mut InchiOutput);
        fn GetStdINCHIKeyFromStdINCHI(source: *const c_char, key: *mut c_char) -> i32;
    }

    pub fn from_molfile(molfile: &str) -> Result<InchiResult, String> {
        let molfile = CString::new(molfile)
            .map_err(|_| "molfile contains an embedded NUL byte".to_string())?;
        let mut options = vec![0 as c_char];
        let mut output = InchiOutput {
            inchi: ptr::null_mut(),
            aux_info: ptr::null_mut(),
            message: ptr::null_mut(),
            log: ptr::null_mut(),
        };
        let return_code = unsafe {
            MakeINCHIFromMolfileText(molfile.as_ptr(), options.as_mut_ptr(), &mut output)
        };
        let inchi = unsafe { optional_string(output.inchi) };
        let auxiliary_info = unsafe { optional_string(output.aux_info) };
        let message = unsafe { optional_string(output.message) };
        let log = unsafe { optional_string(output.log) };
        unsafe { FreeINCHI(&mut output) };
        let inchi = inchi.ok_or_else(|| {
            message
                .clone()
                .or(log)
                .unwrap_or_else(|| format!("InChI generation failed with code {return_code}"))
        })?;
        if !inchi.starts_with("InChI=1S/") {
            return Err(format!(
                "official backend returned a non-standard InChI: {inchi}"
            ));
        }
        let input = CString::new(inchi.as_str()).unwrap();
        let mut key = [0 as c_char; 28];
        let key_code = unsafe { GetStdINCHIKeyFromStdINCHI(input.as_ptr(), key.as_mut_ptr()) };
        if key_code != 0 {
            return Err(format!("InChIKey generation failed with code {key_code}"));
        }
        let inchikey = unsafe { CStr::from_ptr(key.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        Ok(InchiResult {
            inchi,
            inchikey,
            auxiliary_info,
            message,
        })
    }

    unsafe fn optional_string(pointer: *const c_char) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        let value = unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned();
        (!value.is_empty()).then_some(value)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::from_molfile;

#[cfg(target_arch = "wasm32")]
pub fn from_molfile(_molfile: &str) -> Result<InchiResult, String> {
    Err("official InChI is supplied by the browser WebAssembly host".to_string())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn official_backend_generates_water_identifiers() {
        let molfile = "water\n  ChemSema\n\n  1  0  0  0  0  0            999 V2000\n    0.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\nM  END\n";
        let result = from_molfile(molfile).unwrap();
        assert_eq!(result.inchi, "InChI=1S/H2O/h1H2");
        assert_eq!(result.inchikey, "XLYOFNOQVPJJNP-UHFFFAOYSA-N");
    }
}
