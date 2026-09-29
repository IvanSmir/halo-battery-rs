//! HTTPS requests through Windows' own HTTP stack (Windows.Web.Http), so no
//! TLS library is bundled. Needs WinRT initialised on the calling thread.

use windows::Foundation::Uri;
use windows::Web::Http::HttpClient;
use windows::core::HSTRING;

pub struct Response {
    pub status: u16,
    pub body: String,
}

/// GETs `url` with the given extra headers. An error means no answer at all
/// (offline, DNS, TLS); HTTP error statuses come back as a [`Response`].
pub fn get(url: &str, headers: &[(&str, &str)]) -> Result<Response, String> {
    let run = || -> windows::core::Result<Response> {
        let client = HttpClient::new()?;
        let defaults = client.DefaultRequestHeaders()?;
        for (name, value) in headers {
            defaults.TryAppendWithoutValidation(&HSTRING::from(*name), &HSTRING::from(*value))?;
        }
        let response = client.GetAsync(&Uri::CreateUri(&HSTRING::from(url))?)?.join()?;
        let status = response.StatusCode()?.0 as u16;
        let body = response.Content()?.ReadAsStringAsync()?.join()?.to_string();
        Ok(Response { status, body })
    };
    run().map_err(|e| e.message())
}
