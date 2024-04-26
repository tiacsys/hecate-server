use gloo::net::http;
use serde::de::DeserializeOwned;

#[derive(Debug, Copy, Clone)]
pub struct FetchError;

impl<E> From<E> for FetchError
where
    E: std::error::Error,
{
    fn from(_value: E) -> Self {
        Self
    }
}

pub trait Fetch: Sized {
    async fn fetch(url: &str) -> Result<Self, FetchError>;
}

impl<D: DeserializeOwned> Fetch for D {
    async fn fetch(url: &str) -> Result<Self, FetchError> {
        let response = http::Request::get(url).send().await?;
        let parsed = response.json().await?;
        Ok(parsed)
    }
}
