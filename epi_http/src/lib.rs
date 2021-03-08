pub use epi::http::{Request, Response};
#[cfg(feature = "wasm")]
use wasm_bindgen::JsValue;
/// NOTE: Ok(..) is returned on network error.
/// Err is only for failure to use the fetch api.
#[cfg(feature = "wasm")]
pub async fn fetch_async(request: &Request) -> Result<Response, String> {
    fetch_jsvalue(request)
        .await
        .map_err(|err| err.as_string().unwrap_or_default())
}

/// NOTE: Ok(..) is returned on network error.
/// Err is only for failure to use the fetch api.
#[cfg(feature = "wasm")]
async fn fetch_jsvalue(request: &Request) -> Result<Response, JsValue> {
    let Request { method, url, body } = request;

    // https://rustwasm.github.io/wasm-bindgen/examples/fetch.html

    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let mut opts = web_sys::RequestInit::new();
    opts.method(method);
    opts.mode(web_sys::RequestMode::Cors);

    if !body.is_empty() {
        opts.body(Some(&JsValue::from_str(body)));
    }

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

// ----------------------------------------------------------------------------
#[cfg(feature = "wasm")]
pub fn spawn_future<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}
pub struct EpiHttp {}
#[cfg(target_arch = "wasm32")]
impl epi::backend::Http for EpiHttp {
    fn fetch_dyn(
        &self,
        request: Request,
        on_done: Box<dyn FnOnce(Result<Response, String>) + Send>,
    ) {
        spawn_future(async move {
            let result = fetch_async(&request).await;
            on_done(result)
        });
    }
}
#[cfg(feature = "native")]
pub fn fetch_blocking(request: &Request) -> Result<Response, String> {
    let Request { method, url, body } = request;

    let req = ureq::request(method, url).set("Accept", "*/*");
    let resp = if body.is_empty() {
        req.call()
    } else {
        req.set("Content-Type", "text/plain; charset=utf-8")
            .send_string(body)
    };

    let (ok, resp) = match resp {
        Ok(resp) => (true, resp),
        Err(ureq::Error::Status(_, resp)) => (false, resp), // Still read the body on e.g. 404
        Err(ureq::Error::Transport(error)) => return Err(error.to_string()),
    };

    let url = resp.get_url().to_owned();
    let status = resp.status();
    let status_text = resp.status_text().to_owned();
    let header_content_type = resp.header("Content-Type").unwrap_or_default().to_owned();

    let mut reader = resp.into_reader();
    let mut bytes = vec![];
    use std::io::Read;
    reader
        .read_to_end(&mut bytes)
        .map_err(|err| err.to_string())?;

    let text = if header_content_type.starts_with("text")
        || header_content_type == "application/javascript"
    {
        String::from_utf8(bytes.clone()).ok()
    } else {
        None
    };

    let response = Response {
        url,
        ok,
        status,
        status_text,
        header_content_type,
        bytes,
        text,
    };
    Ok(response)
}

#[cfg(not(target_arch = "wasm32"))]
impl epi::backend::Http for EpiHttp {
    fn fetch_dyn(
        &self,
        request: Request,
        on_done: Box<dyn FnOnce(Result<Response, String>) + Send>,
    ) {
        std::thread::spawn(move || {
            let result = fetch_blocking(&request);
            on_done(result)
        });
    }
}
