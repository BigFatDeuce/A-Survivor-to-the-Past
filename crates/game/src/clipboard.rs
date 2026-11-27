//! Presse-papiers (export / import du profil). Repli sur un fichier si le presse-papiers est
//! indisponible.
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub fn set(text: &str) -> bool {
    match arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_string())) {
        Ok(()) => true,
        Err(e) => {
            log::warn!("presse-papiers indisponible : {e}");
            false
        }
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub fn get() -> Option<String> {
    arboard::Clipboard::new().and_then(|mut c| c.get_text()).ok()
}

/// Navigateur : presse-papiers de la page (ecriture asynchrone, lancee sans attendre).
#[cfg(target_arch = "wasm32")]
pub fn set(text: &str) -> bool {
    use wasm_bindgen::{JsCast, JsValue};
    let Some(w) = web_sys::window() else { return false };
    let nav: JsValue = w.navigator().into();
    let Ok(clip) = js_sys::Reflect::get(&nav, &JsValue::from_str("clipboard")) else { return false };
    if clip.is_undefined() {
        return false;
    }
    let Ok(f) = js_sys::Reflect::get(&clip, &JsValue::from_str("writeText")) else { return false };
    let Ok(f) = f.dyn_into::<js_sys::Function>() else { return false };
    f.call1(&clip, &JsValue::from_str(text)).is_ok()
}

/// Navigateur : la lecture du presse-papiers est asynchrone ; on demande au joueur de coller le
/// code dans une boite de saisie.
#[cfg(target_arch = "wasm32")]
pub fn get() -> Option<String> {
    let w = web_sys::window()?;
    w.prompt_with_message("A Survivor to the Past : code (Ctrl+V)").ok().flatten().filter(|s| !s.trim().is_empty())
}

/// Android : pas de presse-papiers (repli sur le fichier, comme quand il est indisponible).
#[cfg(target_os = "android")]
pub fn set(_text: &str) -> bool {
    false
}

#[cfg(target_os = "android")]
pub fn get() -> Option<String> {
    None
}
