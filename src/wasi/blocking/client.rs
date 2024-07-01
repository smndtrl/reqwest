//! blocking-client
use http::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use http::{HeaderName, Method, StatusCode};
use std::convert::{TryFrom, TryInto};
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::Duration;

use super::request::{Request, RequestBuilder};
use super::response::Response;
use crate::error::Kind;
// use crate::wasi::wasi::http::types::RequestOptions;
use super::body::Body;
use crate::{IntoUrl, Proxy}; // Identity, 
use crate::redirect::{self, remove_sensitive_headers};
// use super::wasi::clocks::*;
// use crate::wasi::wasi::http::*;
// use crate::wasi::wasi::io::*;

use spin_sdk::wit::wasi::{
    // io::*,
    http::*,
    http::types::RequestOptions
};

use spin_executor::bindings::wasi::{
    io::*,
};

#[cfg(feature = "__tls")]
use crate::tls::{self, TlsBackend};
#[cfg(feature = "__tls")]
use crate::Certificate;
#[cfg(any(feature = "native-tls", feature = "__rustls"))]
use crate::Identity;

#[derive(Debug)]
struct Config {
    headers: HeaderMap,
    connect_timeout: Option<Duration>,
    timeout: Option<Duration>,
    error: Option<crate::Error>,
}

/// A `Client` to make Requests with.
///
/// The Client has various configuration values to tweak, but the defaults
/// are set to what is usually the most commonly desired value. To configure a
/// `Client`, use `Client::builder()`.
///
/// The `Client` holds a connection pool internally, so it is advised that
/// you create one and **reuse** it.
///
/// # Examples
///
/// ```rust
/// use reqwest::blocking::Client;
/// #
/// # fn run() -> Result<(), reqwest::Error> {
/// let client = Client::new();
/// let resp = client.get("http://httpbin.org/").send()?;
/// #   drop(resp);
/// #   Ok(())
/// # }
///
///
#[derive(Clone, Debug)]
pub struct Client {
    inner: Arc<ClientRef>,
}

#[derive(Debug)]
struct ClientRef {
    pub headers: HeaderMap,
    pub connect_timeout: Option<Duration>,
    pub first_byte_timeout: Option<Duration>,
    pub between_bytes_timeout: Option<Duration>,
}

/// A `ClientBuilder` can be used to create a `Client` with  custom configuration.
///
/// # Example
///
/// ```
/// # fn run() -> Result<(), reqwest::Error> {
/// use std::time::Duration;
///
/// let client = reqwest::blocking::Client::builder()
///     .timeout(Duration::from_secs(10))
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[must_use]
#[derive(Debug)]
pub struct ClientBuilder {
    config: Config,
}

impl Client {
    /// Constructs a new `Client`.
    pub fn new() -> Self {
        Client::builder().build().expect("Client::new()")
    }

    /// Creates a `ClientBuilder` to configure a `Client`.
    ///
    /// This is the same as `ClientBuilder::new()`.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Convenience method to make a `GET` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::GET, url)
    }

    /// Convenience method to make a `POST` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    /// Convenience method to make a `PUT` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    /// Convenience method to make a `PATCH` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    /// Convenience method to make a `DELETE` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    /// Convenience method to make a `HEAD` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::HEAD, url)
    }

    /// Start building a `Request` with the `Method` and `Url`.
    ///
    /// Returns a `RequestBuilder`, which will allow setting headers and
    /// request body before sending.
    ///
    /// # Errors
    ///
    /// This method fails whenever supplied `Url` cannot be parsed.
    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        let req = url.into_url().map(move |url| Request::new(method, url));
        RequestBuilder::new(self.clone(), req)
    }

    /// Executes a `Request`.
    ///
    /// A `Request` can be built manually with `Request::new()` or obtained
    /// from a RequestBuilder with `RequestBuilder::build()`.
    ///
    /// You should prefer to use the `RequestBuilder` and
    /// `RequestBuilder::send()`.
    ///
    /// # Errors
    ///
    /// This method fails if there was an error while sending request,
    /// redirect loop was detected or redirect limit was exhausted.
    pub fn execute(&self, request: Request) -> Result<Response, crate::Error> {
        // tracing::debug!("execute");
        self.execute_inner(request)
    }

    fn execute_inner(&self, request: Request) -> Result<Response, crate::Error> {
        let mut header_key_values: Vec<(String, Vec<u8>)> = vec![];
        for (name, value) in self.inner.headers.iter() {
            match value.to_str() {
                Ok(value) => {
                    header_key_values.push((name.as_str().to_string(), value.as_bytes().to_vec()))
                }
                Err(_) => {}
            }
        }

        let (method, url, headers, body, timeout, _version) = request.pieces();
        for (name, value) in headers.iter() {
            match value.to_str() {
                Ok(value) => {
                    header_key_values.push((name.as_str().to_string(), value.as_bytes().to_vec()))
                }
                Err(_) => {}
            }
        }

        let scheme = match url.scheme() {
            "http" => types::Scheme::Http,
            "https" => types::Scheme::Https,
            other => types::Scheme::Other(other.to_string()),
        };
        let headers = types::Fields::from_list(&header_key_values);
        let path_with_query = match url.query() {
            Some(query) => format!("{}?{}", url.path(), query),
            None => url.path().to_string(),
        };
        // let request = types::OutgoingRequest::new(
        //     &method.into(),
        //     Some(&path_with_query),
        //     Some(&scheme),
        //     Some(url.authority()),
        //     &headers,
        // );

        let request = types::OutgoingRequest::new(headers.expect("header error"));
        let _ = request.set_method(&method.into());
        let _ = request.set_path_with_query(Some(&path_with_query));
        let _ = request.set_scheme(Some(&scheme));
        let _ = request.set_authority(Some(url.authority()));


        // match body {
        //     Some(body) => {
        //         let request_body_stream = request.write().map_err(|e| failure_point("outgoing_request_write", e))?;
        //         body.write(|chunk| {
        //             streams::write(request_body_stream, chunk).map_err(|e| failure_point("write chunk", e))?;
        //             Ok(())
        //         })?;
        //         types::finish_outgoing_stream(request_body_stream, None);
        //         streams::drop_output_stream(request_body_stream);
        //     }
        //     None => {}
        // }

        let outgoing_body = request
            .body()
            .map_err(|e| failure_point("outgoing request write failed", e))?;

        if let Some(body) = body {
            let request_body = outgoing_body
                .write()
                .map_err(|e| failure_point("outgoing request write failed", e))?;

            let pollable = request_body.subscribe();
            // while !buf.is_empty() {
            body.write(|buf| {
                poll::poll(&[&pollable]);

                let permit = match request_body.check_write() {
                    Ok(n) => n,
                    Err(e) => panic!("output stream error: {:?}", e),
                };

                let len = buf.len().min(permit as usize);
                let (chunk, _rest) = buf.split_at(len);
                // buf = rest;

                match request_body.write(chunk) {
                    Err(_) => panic!("output stream error"),
                    _ => Ok(()),
                }
            })?;
            // };

            match request_body.flush() {
                Err(_) => panic!("output stream error"),
                _ => {}
            }

            poll::poll(&[&pollable]);

            match request_body.check_write() {
                Ok(_) => {}
                Err(_) => panic!("output stream error"),
            };
        }

        // let future_incoming_response = outgoing_handler::handle(
        //     request,
        //     Some(types::RequestOptions {
        //         connect_timeout_ms: self.inner.connect_timeout.map(|d| d.as_millis() as u32),
        //         first_byte_timeout_ms: timeout
        //             .or(self.inner.first_byte_timeout)
        //             .map(|d| d.as_millis() as u32),
        //         between_bytes_timeout_ms: timeout
        //             .or(self.inner.between_bytes_timeout)
        //             .map(|d| d.as_millis() as u32),
        //     }),
        // );

        // let receive_timeout = timeout.or(self.inner.first_byte_timeout);
        // let incoming_response =
        //     Self::get_incoming_response(future_incoming_response, receive_timeout)?;

        // let status = types::incoming_response_status(incoming_response);
        // let status_code = StatusCode::from_u16(status)
        //     .map_err(|e| crate::Error::new(crate::error::Kind::Decode, Some(e)))?;

        // let response_fields = types::incoming_response_headers(incoming_response);
        // let response_headers = Self::fields_to_header_map(response_fields);
        // types::drop_fields(headers);
        // types::drop_fields(response_fields);
        // let response_body_stream = types::incoming_response_consume(incoming_response)
        //     .map_err(|e| failure_point("incoming_response_consume", e))?;
        // let response_body: Body = response_body_stream.into();
        let req_opts = RequestOptions::new();
        let _ = req_opts.set_connect_timeout(self.inner.connect_timeout.map(|d| d.as_millis() as u64));
        let _ = req_opts.set_first_byte_timeout( timeout
            .or(self.inner.first_byte_timeout)
            .map(|d| d.as_millis() as u64));
        let _ =  req_opts.set_between_bytes_timeout(timeout
            .or(self.inner.between_bytes_timeout)
            .map(|d| d.as_millis() as u64));

        let future_response = outgoing_handler::handle(
            request,
            Some(req_opts),
        )
        .map_err(|_e| failure_point("outgoing_handler handle", ()))?;


        let _ = types::OutgoingBody::finish(outgoing_body, None);

        let incoming_response = match future_response.get() {
            Some(result) => result.map_err(|_| failure_point("incoming response errored", ()))?,
            None => {
                let pollable = future_response.subscribe();
                let _ = poll::poll(&[&pollable]);
                future_response
                    .get()
                    .expect("incoming response available")
                    .map_err(|e| failure_point("incoming response errored", e))?
            }
        }
        // TODO: maybe anything that appears in the Result<_, E> position should impl
        // Error? anyway, just use its Debug here:
        .map_err(|_e| failure_point("{e:?}", ()))?;

        drop(future_response);

        let status = incoming_response.status();

        let headers_handle = incoming_response.headers();
        let _headers = headers_handle.entries();
        let response_headers = Self::fields_to_header_map(&headers_handle);
        drop(headers_handle);

        let incoming_body = incoming_response
            .consume()
            .map_err(|()| failure_point("incoming response has no body stream", ()))?;

        // drop(incoming_response);

        let input_stream = incoming_body.stream().unwrap();
        let input_stream_pollable = input_stream.subscribe();

        let mut body = Vec::new();
        loop {
            poll::poll(&[&input_stream_pollable]);

            let mut body_chunk = match input_stream.read(1024 * 1024) {
                Ok(c) => c,
                Err(streams::StreamError::Closed) => break,
                Err(_e) => Err(failure_point("input_stream read failed", ()))?,
                // Err(e) => Err(anyhow!("input_stream read failed: {e:?}"))?,
            };

            if !body_chunk.is_empty() {
                body.append(&mut body_chunk);
            }
        }

        let status_code = StatusCode::from_u16(status)
            .map_err(|e| crate::Error::new(crate::error::Kind::Decode, Some(e)))?;

        let response_body = Body::from(body);

        Ok(Response::new(
            status_code,
            response_headers,
            response_body,
            incoming_response,
            url,
        ))
    }

    // fn get_incoming_response(
    //     future_incoming_response: types::FutureIncomingResponse,
    //     timeout: Option<Duration>,
    // ) -> Result<types::IncomingResponse, crate::Error> {
    //     let deadline_pollable = monotonic_clock::subscribe(
    //         timeout.unwrap_or(Duration::from_secs(0)).as_nanos() as u64,
    //         false,
    //     );
    //     loop {
    //         match types::future_incoming_response_get(future_incoming_response) {
    //             Some(Ok(incoming_response)) => {
    //                 types::drop_future_incoming_response(future_incoming_response);
    //                 return Ok(incoming_response);
    //             }
    //             Some(Err(err)) => return Err(err.into()),
    //             None => {
    //                 let pollable =
    //                     types::listen_to_future_incoming_response(future_incoming_response);
    //                 let bitmap = poll::poll_oneoff(&vec![pollable, deadline_pollable]);
    //                 if timeout.is_none() || !bitmap[1] {
    //                     poll::drop_pollable(pollable);
    //                     continue;
    //                 } else {
    //                     return Err(crate::Error::new(
    //                         Kind::Request,
    //                         Some(crate::error::TimedOut),
    //                     ));
    //                 }
    //             }
    //         };
    //     }
    // }

    fn fields_to_header_map(fields: &types::Fields) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let entries = fields.entries();
        for (name, value) in entries {
            headers.insert(
                HeaderName::try_from(&name).expect("Invalid header name"),
                HeaderValue::from_bytes(&value).expect("Invalid header value"),
            );
        }
        headers
    }



}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientBuilder {
    /// Constructs a new `ClientBuilder`.
    ///
    /// This is the same as `Client::builder()`.
    pub fn new() -> Self {
        let mut headers: HeaderMap<HeaderValue> = HeaderMap::with_capacity(2);
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));

        Self {
            config: Config {
                headers,
                connect_timeout: None,
                timeout: None,
                error: None,
            },
        }
    }

    /// Returns a `Client` that uses this `ClientBuilder` configuration.
    ///
    /// # Errors
    ///
    /// This method fails if TLS backend cannot be initialized, or the resolver
    /// cannot load the system configuration.
    pub fn build(self) -> Result<Client, crate::Error> {
        if let Some(err) = self.config.error {
            return Err(err);
        }

        Ok(Client {
            inner: Arc::new(ClientRef {
                headers: self.config.headers,
                connect_timeout: self.config.connect_timeout,
                first_byte_timeout: self.config.timeout,
                between_bytes_timeout: self.config.timeout,
            }),
        })
    }

    /// Sets the `User-Agent` header to be used by this client.
    ///
    /// # Example
    ///
    /// ```rust
    /// # fn doc() -> Result<(), reqwest::Error> {
    /// // Name your user agent after your app?
    /// static APP_USER_AGENT: &str = concat!(
    ///     env!("CARGO_PKG_NAME"),
    ///     "/",
    ///     env!("CARGO_PKG_VERSION"),
    /// );
    ///
    /// let client = reqwest::blocking::Client::builder()
    ///     .user_agent(APP_USER_AGENT)
    ///     .build()?;
    /// let res = client.get("https://www.rust-lang.org").send()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn user_agent<V>(mut self, value: V) -> ClientBuilder
    where
        V: TryInto<HeaderValue>,
        V::Error: Into<http::Error>,
    {
        match value.try_into() {
            Ok(value) => {
                self.config.headers.insert(USER_AGENT, value);
            }
            Err(e) => {
                self.config.error = Some(crate::error::builder(e.into()));
            }
        };
        self
    }

    /// Sets the default headers for every request
    pub fn default_headers(mut self, headers: HeaderMap) -> ClientBuilder {
        for (key, value) in headers.iter() {
            self.config.headers.insert(key, value.clone());
        }
        self
    }

    // TODO: cookie support
    // TODO: gzip support
    // TODO: brotli support
    // TODO: deflate support
    // TODO: redirect support
    // TODO: proxy support
    // TODO: TLS support

    // Timeout options

    /// Set a timeout for connect, read and write operations of a `Client`.
    ///
    /// Default is 30 seconds.
    ///
    /// Pass `None` to disable timeout.
    pub fn timeout<T>(mut self, timeout: T) -> ClientBuilder
    where
        T: Into<Option<Duration>>,
    {
        self.config.timeout = timeout.into();
        self
    }

    /// Set a timeout for only the connect phase of a `Client`.
    ///
    /// Default is `None`.
    pub fn connect_timeout<T>(mut self, timeout: T) -> ClientBuilder
    where
        T: Into<Option<Duration>>,
    {
        self.config.connect_timeout = timeout.into();
        self
    }

    /// Sets the identity to be used for client certificate authentication.
    ///
    /// # Optional
    ///
    /// This requires the optional `native-tls` or `rustls-tls(-...)` feature to be
    /// enabled.
    #[cfg(any(feature = "native-tls", feature = "__rustls"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "native-tls", feature = "rustls-tls"))))]
    #[deprecated(note = "TLS certs and wasi-http not supported")]
    pub fn identity(mut self, _identity: Identity) -> ClientBuilder {
        // self.config.identity = Some(identity);
        self
    }

    /// Placeholder
    // #[deprecated(note = "TLS certs and wasi-http not supported")]
    // pub fn add_root_certificate(mut self, _cert: Certificate) -> ClientBuilder {
    //     // self.config.root_certs.push(cert);
    //     self
    // }

    /// Placeholder
    #[deprecated(note = "TLS certs and wasi-http not supported")]
    pub fn danger_accept_invalid_certs(mut self, _accept_invalid_certs: bool) -> ClientBuilder {
        // self.config.certs_verification = !accept_invalid_certs;
        self
    }

    /// Placeholder
    #[deprecated(note = "Proxy and wasi-http not supported")]
    pub fn proxy(mut self, _proxy: Proxy) -> ClientBuilder {
        // self.config.proxies.push(proxy);
        // self.config.auto_sys_proxy = false;
        self
    }

    /// Placeholder
    #[deprecated(note = "Proxy and wasi-http not supported")]
    pub fn no_proxy(mut self) -> ClientBuilder {
        // self.config.proxies.clear();
        // self.config.auto_sys_proxy = false;
        self
    }

    /// Enable or disable automatic setting of the `Referer` header.
    ///
    /// Default is `true`.
    #[deprecated(note = "Proxy and wasi-http not supported")]
    pub fn referer(mut self, enable: bool) -> ClientBuilder {
        // self.config.referer = enable;
        self
    }

    /// Set whether connections should emit verbose logs.
    ///
    /// Enabling this option will emit [log][] messages at the `TRACE` level
    /// for read and write operations on connections.
    ///
    /// [log]: https://crates.io/crates/log
    #[deprecated(note = "Proxy and wasi-http not supported")]
    pub fn connection_verbose(mut self, verbose: bool) -> ClientBuilder {
        // self.config.connection_verbose = verbose;
        self
    }

    /// Restrict the Client to be used with HTTPS only requests.
    ///
    /// Defaults to false.
    #[deprecated(note = "Proxy and wasi-http not supported")]
    pub fn https_only(mut self, enabled: bool) -> ClientBuilder {
        // self.config.https_only = enabled;
        self
    }

         /// Set the minimum required TLS version for connections.
    ///
    /// By default the TLS backend's own default is used.
    ///
    /// # Errors
    ///
    /// A value of `tls::Version::TLS_1_3` will cause an error with the
    /// `native-tls`/`default-tls` backend. This does not mean the version
    /// isn't supported, just that it can't be set as a minimum due to
    /// technical limitations.
    ///
    /// # Optional
    ///
    /// This requires the optional `default-tls`, `native-tls`, or `rustls-tls(-...)`
    /// feature to be enabled.
    #[cfg(feature = "__tls")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(
            feature = "default-tls",
            feature = "native-tls",
            feature = "rustls-tls"
        )))
    )]
    #[deprecated(note = "Proxy and wasi-http not supported")]
    pub fn min_tls_version(mut self, version: tls::Version) -> ClientBuilder {
        // self.config.min_tls_version = Some(version);
        self
    }

      /// Set a `RedirectPolicy` for this client.
    ///
    /// Default will follow redirects up to a maximum of 10.
    #[deprecated(note = "Proxy and wasi-http not supported")]
    pub fn redirect(mut self, policy: redirect::Policy) -> ClientBuilder {
        // self.config.redirect_policy = policy;
        self
    }
}

// impl From<Method> for types::Method {
//     fn from(value: Method) -> types::Method {
//         if value == Method::GET {
//             types::Method::Get
//         } else if value == Method::POST {
//             types::Method::Post
//         } else if value == Method::PUT {
//             types::Method::Put
//         } else if value == Method::DELETE {
//             types::Method::Delete
//         } else if value == Method::HEAD {
//             types::Method::Head
//         } else if value == Method::OPTIONS {
//             types::Method::Options
//         } else if value == Method::CONNECT {
//             types::Method::Connect
//         } else if value == Method::PATCH {
//             types::Method::Patch
//         } else if value == Method::TRACE {
//             types::Method::Trace
//         } else {
//             types::Method::Other(value.as_str().to_string())
//         }
//     }
// }

// impl From<types::ErrorCode> for crate::Error {
//     fn from(value: types::ErrorCode) -> Self {
//         crate::Error::new(
//             Kind::Request,
//             Some(std::io::Error::new(
//                 ErrorKind::Other,
//                 format!("{:?}", value),
//             )),
//         )
//     }
// }

pub(crate) fn failure_point(s: &str, _: ()) -> crate::Error {
    crate::Error::new(
        Kind::Request,
        Some(std::io::Error::new(ErrorKind::Other, s)),
    )
}
