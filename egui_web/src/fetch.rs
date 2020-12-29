use wasm_bindgen::prelude::*;

pub struct Response {
    pub url: String,
    pub ok: bool,
    pub status: u16,
    pub status_text: String,

    /// Content-Type header, or empty string if missing
    pub header_content_type: String,

    /// The raw bytes
    pub bytes: Vec<u8>,

    /// UTF-8 decoded version of bytes.
    /// ONLY if `header_content_type` starts with "text" and bytes is UTF-8.
    pub text: Option<String>,
}

/// NOTE: Ok(..) is returned on network error.
/// Err is only for failure to use the fetch api.
pub async fn fetch(method: &str, url: &str) -> Result<Response, String> {
    fetch_jsvalue(method, url)
        .await
        .map_err(|err| err.as_string().unwrap_or_default())
}

/// NOTE: Ok(..) is returned on network error.
/// Err is only for failure to use the fetch api.
pub async fn get(url: &str) -> Result<Response, String> {
    fetch("GET", url).await
}

/// NOTE: Ok(..) is returned on network error.
/// Err is only for failure to use the fetch api.
async fn fetch_jsvalue(method: &str, url: &str) -> Result<Response, JsValue> {
    // https://rustwasm.github.io/wasm-bindgen/examples/fetch.html

    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let mut opts = web_sys::RequestInit::new();
    opts.method(method);
    opts.mode(web_sys::RequestMode::Cors);

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("Accept", "*/*")?;

    let window = web_sys::window().unwrap();
    let response = JsFuture::from(window.fetch_with_request(&request)).await?;
    assert!(response.is_instance_of::<web_sys::Response>());
    let response: web_sys::Response = response.dyn_into().unwrap();

    // // TODO: support binary get

    // let body = JsFuture::from(response.text()?).await?;
    // let body = body.as_string().unwrap_or_default();

    let array_buffer = JsFuture::from(response.array_buffer()?).await?;
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    let bytes = uint8_array.to_vec();

    let header_content_type = response
        .headers()
        .get("Content-Type")
        .ok()
        .flatten()
        .unwrap_or_default();

    let text = if header_content_type.starts_with("text")
        || header_content_type == "application/javascript"
    {
        String::from_utf8(bytes.clone()).ok()
    } else {
        None
    };

    Ok(Response {
        status_text: response.status_text(),
        url: response.url(),
        ok: response.ok(),
        status: response.status(),
        header_content_type,
        bytes,
        text,
    })
}
