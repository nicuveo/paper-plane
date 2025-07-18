use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::marker::Sync;
use std::time::{Duration, SystemTime};

use crate::auth::Auth;
use crate::clients::Client as ClientTrait;
use crate::error::{Error, Result};
use crate::response;
use crate::utils::Method;

////////////////////////////////////////////////////////////////////////////////
// Public modules

pub mod lite;

////////////////////////////////////////////////////////////////////////////////
// Public types

#[derive(Debug, Clone)]
pub struct Client {
    inner: reqwest::Client,
    server_url: String,
    auth: Auth,
    additional_headers: Vec<(String, String)>,
}

pub struct Extra {
    pub method: Method,
    pub endpoint: String,
    pub status: reqwest::StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub duration: Duration,
    pub content_type: Option<String>,
}

pub type Response<R> = response::Response<R, Extra>;

////////////////////////////////////////////////////////////////////////////////
// Public implementation

impl Client {
    #[must_use]
    pub fn new(server_url: String, auth: Auth) -> Self {
        Self {
            inner: reqwest::Client::new(),
            server_url,
            auth,
            additional_headers: vec![],
        }
    }

    #[must_use]
    pub fn with_headers(server_url: String, auth: Auth, headers: Vec<(String, String)>) -> Self {
        Self {
            inner: reqwest::Client::new(),
            server_url,
            auth,
            additional_headers: headers,
        }
    }

    #[must_use]
    pub fn additional_headers(&self) -> &[(String, String)] {
        &self.additional_headers
    }

    #[must_use]
    pub fn additional_headers_mut(&mut self) -> &mut Vec<(String, String)> {
        &mut self.additional_headers
    }
}

////////////////////////////////////////////////////////////////////////////////
// Public helpers

#[must_use]
pub fn translate_method(method: Method) -> reqwest::Method {
    match method {
        Method::GET => reqwest::Method::GET,
        Method::PUT => reqwest::Method::PUT,
        Method::POST => reqwest::Method::POST,
        Method::PATCH => reqwest::Method::PATCH,
        Method::DELETE => reqwest::Method::DELETE,
    }
}

////////////////////////////////////////////////////////////////////////////////
// Internal helpers

impl Client {
    fn build<P, B>(
        &self,
        method: Method,
        endpoint: &str,
        params: &P,
        body: Option<&B>,
    ) -> Result<reqwest::Request>
    where
        P: Serialize,
        B: Serialize,
    {
        let uri = format!("{}{endpoint}", self.server_url);
        let mut request = self
            .inner
            .request(translate_method(method), &uri)
            .header(reqwest::header::ACCEPT, "application/json; version=9")
            .header(reqwest::header::AUTHORIZATION, self.auth.header_value())
            .query(params);
        if let Some(body) = body {
            request = request.json(body);
        }
        for (header_name, header_value) in &self.additional_headers {
            request = request.header(header_name, header_value);
        }
        request.build().map_err(|e| Error::RequestBuild {
            method,
            endpoint: endpoint.to_string(),
            source: e.into(),
        })
    }
}

////////////////////////////////////////////////////////////////////////////////
// Traits

#[async_trait]
impl ClientTrait for Client {
    type Extra = Extra;

    async fn request_json<P, B, R>(
        &self,
        method: Method,
        endpoint: &str,
        params: &P,
        body: Option<&B>,
    ) -> Result<Response<R>>
    where
        B: Serialize + Sync,
        P: Serialize + Sync,
        R: for<'a> Deserialize<'a>,
    {
        let request = self.build(method, endpoint, params, body)?;
        let start = SystemTime::now();
        let resp = self
            .inner
            .execute(request)
            .await
            .map_err(|source| Error::RequestSend {
                method,
                endpoint: endpoint.to_string(),
                source: source.into(),
            })?;
        let duration = start.elapsed().unwrap_or(Duration::from_secs(0));
        let status = resp.status();
        let headers = resp.headers().clone();
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .map(String::from);

        if let Err(source) = resp.error_for_status_ref() {
            let content = match resp.text().await {
                Err(_) => serde_json::Value::String("<failed to retrieve content>".to_string()),
                Ok(content) => {
                    serde_json::from_str(&content).unwrap_or(serde_json::Value::String(content))
                }
            };
            return Err(Error::Server {
                method,
                endpoint: endpoint.to_string(),
                status: format!("{status}"),
                content,
                source: source.into(),
            });
        }

        if content_type != Some("application/json".to_string()) {
            return Err(Error::ContentType {
                method,
                endpoint: endpoint.to_string(),
                expected: vec!["application/json".to_string()],
                received: content_type,
            });
        }

        let content = resp.text().await.map_err(|source| Error::ResponseBody {
            method,
            endpoint: endpoint.to_string(),
            source: source.into(),
        })?;

        Ok(Response {
            value: serde_json::from_str(&content).map_err(|source| Error::Deserializing {
                method,
                endpoint: endpoint.to_string(),
                typename: std::any::type_name::<R>(),
                content,
                source,
            })?,
            extra: Extra {
                method,
                endpoint: endpoint.to_string(),
                status,
                headers,
                duration,
                content_type,
            },
        })
    }

    async fn request_bytes<P, B>(
        &self,
        method: Method,
        endpoint: &str,
        params: &P,
        body: Option<&B>,
    ) -> Result<Response<Bytes>>
    where
        P: Serialize + Sync,
        B: Serialize + Sync,
    {
        let request = self.build(method, endpoint, params, body)?;
        let start = SystemTime::now();
        let resp = self
            .inner
            .execute(request)
            .await
            .map_err(|source| Error::RequestSend {
                method,
                endpoint: endpoint.to_string(),
                source: source.into(),
            })?;
        let duration = start.elapsed().unwrap_or(Duration::from_secs(0));
        let status = resp.status();
        let headers = resp.headers().clone();
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .map(String::from);

        if let Err(source) = resp.error_for_status_ref() {
            let content = match resp.text().await {
                Err(_) => serde_json::Value::String("<failed to retrieve content>".to_string()),
                Ok(content) => {
                    serde_json::from_str(&content).unwrap_or(serde_json::Value::String(content))
                }
            };
            return Err(Error::Server {
                method,
                endpoint: endpoint.to_string(),
                status: format!("{status}"),
                content,
                source: source.into(),
            });
        }

        Ok(Response {
            value: resp.bytes().await.map_err(|source| Error::ResponseBody {
                method,
                endpoint: endpoint.to_string(),
                source: source.into(),
            })?,
            extra: Extra {
                method,
                endpoint: endpoint.to_string(),
                status,
                headers,
                duration,
                content_type,
            },
        })
    }

    async fn request_unit<P, B>(
        &self,
        method: Method,
        endpoint: &str,
        params: &P,
        body: Option<&B>,
    ) -> Result<Response<()>>
    where
        P: Serialize + Sync,
        B: Serialize + Sync,
    {
        let request = self.build(method, endpoint, params, body)?;
        let start = SystemTime::now();
        let resp = self
            .inner
            .execute(request)
            .await
            .map_err(|source| Error::RequestSend {
                method,
                endpoint: endpoint.to_string(),
                source: source.into(),
            })?;
        let duration = start.elapsed().unwrap_or(Duration::from_secs(0));
        let status = resp.status();
        let headers = resp.headers().clone();
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .map(String::from);

        if let Err(source) = resp.error_for_status_ref() {
            let content = match resp.text().await {
                Err(_) => serde_json::Value::String("<failed to retrieve content>".to_string()),
                Ok(content) => {
                    serde_json::from_str(&content).unwrap_or(serde_json::Value::String(content))
                }
            };
            return Err(Error::Server {
                method,
                endpoint: endpoint.to_string(),
                status: format!("{status}"),
                content,
                source: source.into(),
            });
        }

        Ok(Response {
            value: (),
            extra: Extra {
                method,
                endpoint: endpoint.to_string(),
                status,
                headers,
                duration,
                content_type,
            },
        })
    }
}
