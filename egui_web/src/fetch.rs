use wasm_bindgen::prelude::*;

pub struct Response {
    pub url: String,
    pub ok: bool,
    pub status: u16,
    pub status_text: String,

    pub body: String,
}

/// NOTE: Ok(..) is returned on network error.
/// Err is only for failure to use the fetch api.
pub async fn get_text(url: &str) -> Result<Response, String> {
    get_text_jsvalue(url)
        .await
        .map_err(|err| err.as_string().unwrap_or_default())
}

/// NOTE: Ok(..) is returned on network error.
/// Err is only for failure to use the fetch api.
async fn get_text_jsvalue(url: &str) -> Result<Response, JsValue> {
    // https://rustwasm.github.io/wasm-bindgen/examples/fetch.html

    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let mut opts = web_sys::RequestInit::new();
    opts.method("GET");
    opts.mode(web_sys::RequestMode::Cors);

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("Accept", "*/*")?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    assert!(resp_value.is_instance_of::<web_sys::Response>());
    let resp: web_sys::Response = resp_value.dyn_into().unwrap();

    // TODO: headers
    // TODO: support binary get
    let body = JsFuture::from(resp.text()?).await?;
    let body = body.as_string().unwrap_or_default();

    Ok(Response {
        status_text: resp.status_text(),
        url: resp.url(),
        ok: resp.ok(),
        status: resp.status(),
        body,
    })
}
