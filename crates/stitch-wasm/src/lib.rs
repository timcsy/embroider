//! 瀏覽器綁定層：只做 JS 與 Rust 型別之間的轉接，不放業務邏輯。

pub mod api;

use std::cell::RefCell;
use wasm_bindgen::prelude::*;

thread_local! {
    /// 字型在載入時註冊一次，之後的函式都以此為輸入。
    static FONTS: RefCell<api::FontSet> = RefCell::new(api::FontSet::default());
}

fn out<T: serde::Serialize>(v: &T) -> Result<JsValue, JsError> {
    // 缺值一律轉成 null（而不是 undefined），與前端型別一致
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    v.serialize(&serializer)
        .map_err(|e| JsError::new(&e.to_string()))
}

fn input<T: serde::de::DeserializeOwned>(v: JsValue) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(v).map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen(js_name = listBuiltinHoops)]
pub fn list_builtin_hoops() -> Result<JsValue, JsError> {
    out(&api::list_builtin_hoops())
}

#[wasm_bindgen(js_name = listFormats)]
pub fn list_formats() -> Result<JsValue, JsError> {
    out(&api::list_formats())
}

#[wasm_bindgen(js_name = validateHoop)]
pub fn validate_hoop(hoop: JsValue) -> Result<JsValue, JsError> {
    out(&api::validate_hoop(&input(hoop)?))
}

/// 註冊字型。回傳 `{ ok, message }`。
#[wasm_bindgen(js_name = registerFont)]
pub fn register_font(id: &str, name: &str, bytes: &[u8]) -> Result<JsValue, JsError> {
    let result = FONTS.with(|f| f.borrow_mut().add(id, name, bytes.to_vec()));
    out(&api::HoopValidation {
        ok: result.is_ok(),
        message: result.err(),
    })
}

#[wasm_bindgen(js_name = listFonts)]
pub fn list_fonts() -> Result<JsValue, JsError> {
    FONTS.with(|f| out(&api::list_fonts(&f.borrow())))
}

#[wasm_bindgen(js_name = buildTextPreview)]
pub fn build_text_preview(request: JsValue) -> Result<JsValue, JsError> {
    let req: api::TextRequest = input(request)?;
    FONTS.with(|f| out(&api::build_text_preview(&f.borrow(), &req)))
}

/// 圖片轉刺繡。像素另外以 Uint8Array 傳入（RGBA），其餘設定放在 request。
#[wasm_bindgen(js_name = buildImagePreview)]
pub fn build_image_preview(request: JsValue, rgba: &[u8]) -> Result<JsValue, JsError> {
    let mut req: api::ImageRequest = input(request)?;
    req.rgba = rgba.to_vec();
    out(&api::build_image_preview(&req))
}

#[wasm_bindgen(js_name = suggestColorCount)]
pub fn suggest_color_count(rgba: &[u8], width: usize, height: usize) -> usize {
    api::suggest_color_count(rgba, width, height)
}

#[wasm_bindgen(js_name = listThreadColors)]
pub fn list_thread_colors() -> Result<JsValue, JsError> {
    out(&api::list_thread_colors())
}

/// 輸出多種格式。回傳 `{ files: [{ fileName, bytes: Uint8Array }] | null, error: UserMessage | null }`。
#[wasm_bindgen(js_name = encodeAll)]
pub fn encode_all(
    plan: JsValue,
    hoop: JsValue,
    format_ids: JsValue,
    design_name: &str,
    sequence: u32,
) -> Result<JsValue, JsError> {
    let plan = input(plan)?;
    let hoop = input(hoop)?;
    let ids: Vec<String> = input(format_ids)?;
    let result = js_sys::Object::new();
    let set = |k: &str, v: JsValue| js_sys::Reflect::set(&result, &JsValue::from_str(k), &v);
    match api::encode_all(&plan, &hoop, &ids, design_name, sequence) {
        Ok(files) => {
            let arr = js_sys::Array::new();
            for f in files {
                let o = js_sys::Object::new();
                js_sys::Reflect::set(&o, &"fileName".into(), &f.file_name.into()).ok();
                js_sys::Reflect::set(&o, &"bytes".into(), &js_sys::Uint8Array::from(&f.bytes[..]))
                    .ok();
                arr.push(&o);
            }
            set("files", arr.into()).ok();
            set("error", JsValue::NULL).ok();
        }
        Err(m) => {
            set("files", JsValue::NULL).ok();
            set("error", out(&m)?).ok();
        }
    }
    Ok(result.into())
}
