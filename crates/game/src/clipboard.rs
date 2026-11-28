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

/// Navigateur : presse-papiers de la page. L'ecriture est asynchrone : si le navigateur la refuse
/// ensuite (permission, page sans activation), le code s'affiche dans une boite d'ou le copier.
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
    let Ok(promise) = f.call1(&clip, &JsValue::from_str(text)) else { return false };
    let code = text.to_string();
    let refused = wasm_bindgen::closure::Closure::once_into_js(move |_: JsValue| {
        log::warn!("presse-papiers refuse : code affiche a copier");
        if let Some(w) = web_sys::window() {
            let _ = w.prompt_with_message_and_default("A Survivor to the Past : code (Ctrl+C)", &code);
        }
    });
    if let Some(catch) = js_sys::Reflect::get(&promise, &JsValue::from_str("catch")).ok().and_then(|c| c.dyn_into::<js_sys::Function>().ok()) {
        let _ = catch.call1(&promise, &refused);
    }
    true
}

/// Navigateur : la lecture du presse-papiers est asynchrone ; on demande au joueur de coller le
/// code dans une boite de saisie.
#[cfg(target_arch = "wasm32")]
pub fn get() -> Option<String> {
    let w = web_sys::window()?;
    w.prompt_with_message("A Survivor to the Past : code (Ctrl+V)").ok().flatten().filter(|s| !s.trim().is_empty())
}

/// Android : ClipboardManager du systeme, par JNI (NativeActivity, pas de code Java).
#[cfg(target_os = "android")]
pub fn set(text: &str) -> bool {
    use jni::{jni_sig, jni_str};
    android_clipboard(|env, _, clip| {
        let label = env.new_string("A Survivor to the Past")?;
        let text = env.new_string(text)?;
        let data = env
            .call_static_method(
                jni_str!("android/content/ClipData"),
                jni_str!("newPlainText"),
                jni_sig!("(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;"),
                &[(&label).into(), (&text).into()],
            )?
            .l()?;
        env.call_method(clip, jni_str!("setPrimaryClip"), jni_sig!("(Landroid/content/ClipData;)V"), &[(&data).into()])?;
        Ok(true)
    })
    .unwrap_or(false)
}

#[cfg(target_os = "android")]
pub fn get() -> Option<String> {
    use jni::{jni_sig, jni_str, objects::JString, JValue};
    android_clipboard(|env, context, clip| {
        let data = env.call_method(clip, jni_str!("getPrimaryClip"), jni_sig!("()Landroid/content/ClipData;"), &[])?;
        if data.is_null() {
            return Ok(None);
        }
        let data = data.l()?;
        if env.call_method(&data, jni_str!("getItemCount"), jni_sig!("()I"), &[])?.i()? < 1 {
            return Ok(None);
        }
        let item = env.call_method(&data, jni_str!("getItemAt"), jni_sig!("(I)Landroid/content/ClipData$Item;"), &[JValue::Int(0)])?.l()?;
        let cs = env
            .call_method(&item, jni_str!("coerceToText"), jni_sig!("(Landroid/content/Context;)Ljava/lang/CharSequence;"), &[context.into()])?
            .l()?;
        let s = env.call_method(&cs, jni_str!("toString"), jni_sig!("()Ljava/lang/String;"), &[])?.l()?;
        let s = unsafe { JString::from_raw(env, s.into_raw()) };
        Ok(Some(s.try_to_string(env)?).filter(|s| !s.trim().is_empty()))
    })
    .flatten()
}

/// Appelle `f` avec le contexte de l'application et son ClipboardManager ; une erreur Java est
/// effacee et donne `None` (repli sur le fichier).
#[cfg(target_os = "android")]
fn android_clipboard<R>(
    f: impl for<'j> FnOnce(&mut jni::Env<'j>, &jni::objects::JObject<'j>, &jni::objects::JObject<'j>) -> jni::errors::Result<R>,
) -> Option<R> {
    use jni::{jni_sig, jni_str, objects::JObject, JavaVM};
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) };
    let r = vm.attach_current_thread(|env| {
        let run = |env: &mut jni::Env<'_>| {
            let context = unsafe { JObject::from_raw(env, ctx.context().cast()) };
            let name = env.new_string("clipboard")?;
            let clip = env
                .call_method(&context, jni_str!("getSystemService"), jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"), &[(&name).into()])?
                .l()?;
            f(env, &context, &clip)
        };
        let r = run(env);
        if env.exception_check() {
            env.exception_clear();
        }
        r
    });
    r.map_err(|e| log::warn!("presse-papiers Android : {e}")).ok()
}
