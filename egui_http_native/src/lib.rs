#[cfg(feature = "http")]
pub use epi::http::{Request, Response};

/// NOTE: Ok(..) is returned on network error.
/// Err is only for failure to use the fetch api.
#[cfg(feature = "http")]
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

// ----------------------------------------------------------------------------
#[cfg(feature = "http")]
pub struct EguiHttpNative {}
#[cfg(feature = "http")]
impl epi::backend::Http for EguiHttpNative {
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
