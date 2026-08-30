use h2::ext::{PseudoOrder, StreamPriority};
use log::debug;
use reqwest::{cookie::CookieStore, header::HeaderMap, Method, Response, Version};
use std::{fmt::Debug, net::IpAddr, str::FromStr, sync::Arc, time::Duration};
use url::Url;

use crate::{
    errors::{ErrorContext, ImpitError},
    fingerprint::BrowserFingerprint,
    http_headers::HttpHeaders,
    request::{ImpitBody, ImpitRequest, RequestOptions},
    tls,
};

/// Impit is the main struct used to make (impersonated) requests.
///
/// It uses `reqwest::Client` to make requests and holds info about the impersonated browser.
///
/// To create a new [`Impit`] instance, use the [`Impit::builder()`](ImpitBuilder) method.
pub struct Impit<CookieStoreImpl: CookieStore + 'static> {
    pub(self) base_client: reqwest::Client,
    pub(self) vanilla_client: Option<reqwest::Client>,
    /// The HTTP/2 pseudo-header order this client writes, taken from the
    /// fingerprint.
    ///
    /// Carried as a field and attached to each request's extensions rather
    /// than exported through `IMPIT_H2_PSEUDOHEADERS_ORDER`, which is what
    /// upstream does. `std::env::set_var` is unsafe in Rust 2024, is
    /// undefined behaviour beside another thread reading the environment, and
    /// is process global, so two clients with different fingerprints in one
    /// process overwrite each other. See patches/UPSTREAM.md.
    pseudo_header_order: Option<PseudoOrder>,
    config: ImpitBuilder<CookieStoreImpl>,
}

struct PreparedRequest {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: ImpitBody,
}

impl<CookieStoreImpl: CookieStore + 'static> Default for Impit<CookieStoreImpl> {
    fn default() -> Self {
        ImpitBuilder::<CookieStoreImpl>::default().build().unwrap()
    }
}

/// Customizes the behavior of the [`Impit`] struct when following redirects.
///
/// The `RedirectBehavior` enum is used to specify how the client should handle redirects.
#[derive(Debug, Clone)]
pub enum RedirectBehavior {
    /// Follow up to `usize` redirects.
    ///
    /// If the number of redirects is exceeded, the client will return an error.
    FollowRedirect(usize),
    /// Don't follow any redirects.
    ///
    /// The client will return the response for the first request, even with the `3xx` status code.
    ManualRedirect,
}

/// A builder struct used to create a new [`Impit`] instance.
///
/// The builder allows setting the browser to impersonate, ignoring TLS errors, setting a proxy, and other options.
///
/// ### Example
/// ```rust,no_run
/// use impit::{impit::Impit, fingerprint::database as fingerprints};
/// use reqwest::cookie::Jar;
/// use std::time::Duration;
///
/// # #[tokio::main]
/// # async fn main() {
/// let impit = Impit::<Jar>::builder()
///   .with_fingerprint(fingerprints::firefox_144::fingerprint())
///   .with_ignore_tls_errors(true)
///   .with_proxy("http://localhost:8080".to_string())
///   .with_default_timeout(Duration::from_secs(10))
///   .build()
///   .unwrap();
///
/// let response = impit.get("https://example.com".to_string(), None, None).await;
/// # }
/// ```
#[derive(Debug)]
pub struct ImpitBuilder<CookieStoreImpl: CookieStore + 'static> {
    fingerprint: Option<BrowserFingerprint>,
    ignore_tls_errors: bool,
    vanilla_fallback: bool,
    proxy_url: String,
    request_timeout: Duration,
    max_http_version: Version,
    redirect: RedirectBehavior,
    cookie_store: Option<Arc<CookieStoreImpl>>,
    headers: Option<Vec<(String, String)>>,
    local_address: Option<IpAddr>,
    extra_roots: Vec<Vec<u8>>,
    /// `SETTINGS_HEADER_TABLE_SIZE`, which the fingerprint database does not
    /// carry and a current Chrome does send.
    http2_header_table_size: Option<u32>,
    /// Whether to leave `SETTINGS_MAX_FRAME_SIZE` out of the SETTINGS frame.
    /// A current Chrome does not send it and `hyper` always does.
    http2_omit_max_frame_size: bool,
    /// The PRIORITY block to open the first stream with, which the fingerprint
    /// database does not carry and a current Chrome does send.
    http2_stream_priority: Option<StreamPriority>,
}

impl<CookieStoreImpl: CookieStore + 'static> Clone for ImpitBuilder<CookieStoreImpl> {
    fn clone(&self) -> Self {
        ImpitBuilder {
            fingerprint: self.fingerprint.clone(),
            ignore_tls_errors: self.ignore_tls_errors,
            vanilla_fallback: self.vanilla_fallback,
            proxy_url: self.proxy_url.clone(),
            request_timeout: self.request_timeout,
            max_http_version: self.max_http_version,
            redirect: self.redirect.clone(),
            cookie_store: self.cookie_store.clone(),
            headers: self.headers.clone(),
            local_address: self.local_address,
            extra_roots: self.extra_roots.clone(),
            http2_header_table_size: self.http2_header_table_size,
            http2_omit_max_frame_size: self.http2_omit_max_frame_size,
            http2_stream_priority: self.http2_stream_priority,
        }
    }
}

impl<CookieStoreImpl: CookieStore + 'static> Default for ImpitBuilder<CookieStoreImpl> {
    fn default() -> Self {
        ImpitBuilder {
            fingerprint: None,
            ignore_tls_errors: false,
            vanilla_fallback: false,
            proxy_url: String::new(),
            request_timeout: Duration::from_secs(30),
            max_http_version: Version::HTTP_2,
            redirect: RedirectBehavior::FollowRedirect(10),
            cookie_store: None,
            headers: None,
            local_address: None,
            extra_roots: Vec::new(),
            http2_header_table_size: None,
            http2_omit_max_frame_size: false,
            http2_stream_priority: None,
        }
    }
}

impl<CookieStoreImpl: CookieStore + 'static> ImpitBuilder<CookieStoreImpl> {
    /// Sets a complete browser fingerprint.
    ///
    /// This method allows you to provide a complete fingerprint that includes TLS, HTTP/2, and HTTP header configurations.
    /// When set, this takes precedence over the `with_browser` method.
    ///
    /// You can use pre-defined fingerprints from [`crate::fingerprint::database`] or create custom fingerprints.
    pub fn with_fingerprint(mut self, fingerprint: BrowserFingerprint) -> Self {
        self.fingerprint = Some(fingerprint);
        self
    }

    /// If set to true, the client will ignore TLS-related errors.
    pub fn with_ignore_tls_errors(mut self, ignore_tls_errors: bool) -> Self {
        self.ignore_tls_errors = ignore_tls_errors;
        self
    }

    /// If set to `true`, the client will retry the request without impersonation
    /// if the impersonated browser encounters an error.
    pub fn with_fallback_to_vanilla(mut self, vanilla_fallback: bool) -> Self {
        self.vanilla_fallback = vanilla_fallback;
        self
    }

    /// Sets the proxy URL to use for requests.
    ///
    /// Note that this proxy will be used for all the requests
    /// made by the built [`Impit`] instance.
    pub fn with_proxy(mut self, proxy_url: String) -> Self {
        self.proxy_url = proxy_url;
        self
    }

    /// Sets the default timeout for requests.
    ///
    /// This setting can be overridden when making the request by using the `RequestOptions` struct.
    pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Sets the desired redirect behavior.
    ///
    /// By default, the client will follow up to 10 redirects.
    /// By passing the `RedirectBehavior::ManualRedirect` option, the client will not follow any redirects
    /// (i.e. it will return the response for the first request, with the 3xx status code).
    pub fn with_redirect(mut self, behavior: RedirectBehavior) -> Self {
        self.redirect = behavior;
        self
    }

    /// Sets whether to store cookies in the internal Client cookie store.
    ///
    /// If set to `true`, the client will store cookies in the internal cookie store.
    /// If set to `false`, the client will not store cookies. Response headers will contain the
    /// `Set-Cookie` header.
    pub fn with_cookie_store(mut self, cookie_store: CookieStoreImpl) -> Self {
        self.cookie_store = Some(Arc::new(cookie_store));
        self
    }

    /// Sets the local address to bind the client to.
    ///
    /// This is useful for testing purposes or when you want to bind the client to a specific network interface.
    /// Note that this address should be a valid IP address in the format "xxx.xxx.xxx.xxx" (for IPv4) or
    /// "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff" (for IPv6).
    pub fn with_local_address(mut self, local_address: String) -> Result<Self, ImpitError> {
        let ip_addr = local_address.parse::<IpAddr>().map_err(|_| {
            ImpitError::ReqwestError(format!("Invalid local address: {local_address}"))
        })?;

        self.local_address = Some(ip_addr);
        Ok(self)
    }

    /// Sets additional headers to include in every request made by the built [`Impit`] instance.
    ///
    /// This can be used to add e.g. custom user-agent or authorization headers that should be included in every request.
    /// These headers override the "impersonation" headers set by the `with_browser` method.
    ///
    /// If you want to add custom headers to a specific request, use the `RequestOptions` struct instead.
    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Announces `SETTINGS_HEADER_TABLE_SIZE` on HTTP/2 connections.
    ///
    /// Added by this repository; see patches/UPSTREAM.md.
    pub fn with_http2_header_table_size(mut self, size: Option<u32>) -> Self {
        self.http2_header_table_size = size;
        self
    }

    /// Leaves `SETTINGS_MAX_FRAME_SIZE` out of the SETTINGS frame.
    ///
    /// Added by this repository; see patches/UPSTREAM.md.
    pub fn with_http2_omit_max_frame_size(mut self, omit: bool) -> Self {
        self.http2_omit_max_frame_size = omit;
        self
    }

    /// Opens the first stream with a PRIORITY block, the way a browser does.
    ///
    /// It is here rather than on [`crate::fingerprint::Http2Fingerprint`] for
    /// the same reason the two settings above are: the database does not carry
    /// the value and putting it there would edit twenty-seven profile literals
    /// that do not use it, which is a reconciliation cost for nothing.
    ///
    /// Added by this repository; see patches/UPSTREAM.md.
    pub fn with_http2_stream_priority(mut self, priority: Option<StreamPriority>) -> Self {
        self.http2_stream_priority = priority;
        self
    }

    /// Trusts these DER certificate authorities as well as the usual roots.
    ///
    /// Added rather than substituted, and nothing here disables verification.
    /// See `crate::tls`. Added by this repository; see patches/UPSTREAM.md.
    pub fn with_extra_roots(mut self, extra_roots: Vec<Vec<u8>>) -> Self {
        self.extra_roots = extra_roots;
        self
    }

    /// Builds the [`Impit`] instance.
    pub fn build(self) -> Result<Impit<CookieStoreImpl>, ImpitError> {
        Impit::new(self)
    }
}

impl<CookieStoreImpl: CookieStore + 'static> Impit<CookieStoreImpl> {
    pub fn builder() -> ImpitBuilder<CookieStoreImpl> {
        ImpitBuilder::default()
    }

    fn new_reqwest_client(
        config: &ImpitBuilder<CookieStoreImpl>,
    ) -> Result<reqwest::Client, ImpitError> {
        let mut client = reqwest::Client::builder();
        let mut tls_config_builder = tls::TlsConfig::builder();

        // Use fingerprint if provided, otherwise fall back to browser enum
        if let Some(ref fingerprint) = config.fingerprint {
            tls_config_builder.with_tls_fingerprint(fingerprint.tls.clone());

            if let Some(window_size) = fingerprint.http2.initial_stream_window_size {
                client = client.http2_initial_stream_window_size(window_size);
            }

            if let Some(window_size) = fingerprint.http2.initial_connection_window_size {
                client = client.http2_initial_connection_window_size(window_size);
            }

            if let Some(max_size) = fingerprint.http2.max_header_list_size {
                client = client.http2_max_header_list_size(max_size);
            }

            if let Some(size) = config.http2_header_table_size {
                client = client.http2_header_table_size(size);
            }

            client = client.http2_omit_max_frame_size(config.http2_omit_max_frame_size);
        }

        tls_config_builder.with_ignore_tls_errors(config.ignore_tls_errors);
        if !config.extra_roots.is_empty() {
            tls_config_builder.with_extra_roots(config.extra_roots.clone());
        }

        let tls_config = tls_config_builder.build();

        client = client
            .danger_accept_invalid_certs(config.ignore_tls_errors)
            .danger_accept_invalid_hostnames(config.ignore_tls_errors)
            .use_preconfigured_tls(tls_config)
            .timeout(config.request_timeout);

        if let Some(cookie_provider) = &config.cookie_store {
            client = client.cookie_provider(cookie_provider.clone());
        }

        if !config.proxy_url.is_empty() {
            client = client.proxy(
                reqwest::Proxy::all(&config.proxy_url)
                    .map_err(|_| ImpitError::ProxyError(config.proxy_url.clone()))?,
            );
        }

        if let Some(ip_addr) = config.local_address {
            client = client.local_address(ip_addr);
        }

        match config.redirect {
            RedirectBehavior::FollowRedirect(max) => {
                client = client.redirect(reqwest::redirect::Policy::limited(max));
            }
            RedirectBehavior::ManualRedirect => {
                client = client.redirect(reqwest::redirect::Policy::none());
            }
        }

        client
            .build()
            .map_err(|e| ImpitError::ReqwestError(format!("{e:#?}")))
    }

    /// Creates a new [`Impit`] instance based on the options stored in the [`ImpitBuilder`] instance.
    fn new(config: ImpitBuilder<CookieStoreImpl>) -> Result<Self, ImpitError> {
        let base_client = Self::new_reqwest_client(&config)?;

        let vanilla_client = if config.vanilla_fallback && config.fingerprint.is_some() {
            Some(Self::new_reqwest_client(
                &ImpitBuilder::<CookieStoreImpl> {
                    fingerprint: None,
                    max_http_version: Version::HTTP_2,
                    ..config.clone()
                },
            )?)
        } else {
            None
        };

        // The pseudo-header order comes off the fingerprint and is carried on
        // this client rather than exported to the process. A list naming
        // anything but the six pseudo-headers, or omitting one, is refused
        // here rather than panicking inside the HTTP/2 encoder.
        let pseudo_header_order = match config.fingerprint.as_ref() {
            Some(fingerprint) if !fingerprint.http2.pseudo_header_order.is_empty() => {
                let text = fingerprint.http2.pseudo_header_order.join(",");
                Some(PseudoOrder::parse(&text).map_err(|e| {
                    ImpitError::BindingPassthroughError(format!(
                        "the fingerprint for {} {} names a pseudo-header order that cannot be used: {e}",
                        fingerprint.name, fingerprint.version
                    ))
                })?)
            }
            _ => None,
        };

        Ok(Impit {
            base_client,
            vanilla_client,
            pseudo_header_order,
            config,
        })
    }

    fn parse_url(&self, url: String) -> Result<Url, ImpitError> {
        let url = Url::parse(&url).map_err(|_| ImpitError::UrlParsingError(url.clone()))?;

        if url.host_str().is_none() {
            return Err(ImpitError::UrlMissingHostnameError(url.to_string()));
        }

        let protocol = url.scheme();

        match protocol {
            "http" => Ok(url),
            "https" => Ok(url),
            _ => Err(ImpitError::UrlProtocolError(protocol.to_string())),
        }
    }

    fn build_request(
        &self,
        method: Method,
        url: Url,
        body: Option<ImpitBody>,
        headers: Vec<(String, String)>,
    ) -> ImpitRequest {
        let host = url.host_str().unwrap_or_default().to_string();

        let headers = HttpHeaders::get_builder()
            .with_fingerprint(&self.config.fingerprint)
            .with_host(&host)
            .with_https(url.scheme() == "https")
            .with_custom_headers(self.config.headers.to_owned())
            .with_custom_headers(Some(headers))
            .build();

        ImpitRequest {
            url,
            body: body.unwrap_or_default(),
            headers: headers.iter().collect(),
            method: method.to_string(),
        }
    }

    async fn execute_request(
        &self,
        client: &reqwest::Client,
        prepared: &mut PreparedRequest,
        timeout: Option<Duration>,
    ) -> Result<Response, reqwest::Error> {
        let mut req = client
            .request(prepared.method.clone(), prepared.url.clone())
            .headers(prepared.headers.clone());

        if let Some(t) = timeout {
            req = req.timeout(t);
        }

        if let Some(body) = prepared.body.take() {
            req = req.body(body);
        }

        let stream_priority = self.config.http2_stream_priority;
        if self.pseudo_header_order.is_none() && stream_priority.is_none() {
            return req.send().await;
        }

        // reqwest keeps its own per-request settings in the same extension
        // map and does not expose it, so what `h2` has to read goes on through
        // the two public conversions rather than through a private field. Both
        // are lossless, so the timeout set above survives the round trip and
        // reaches `h2` beside ours. See patches/UPSTREAM.md.
        let request = req.build()?;
        let mut http_request: http::Request<reqwest::Body> = request.try_into()?;
        if let Some(pseudo_order) = self.pseudo_header_order {
            http_request.extensions_mut().insert(pseudo_order);
        }
        if let Some(priority) = stream_priority {
            http_request.extensions_mut().insert(priority);
        }
        let request: reqwest::Request = http_request.try_into()?;
        client.execute(request).await
    }

    async fn send(
        &self,
        request: ImpitRequest,
        timeout: Option<Duration>,
        http3_prior_knowledge: Option<bool>,
    ) -> Result<Response, ImpitError> {
        // HTTP/3 is not built. Asking for it by prior knowledge is an error
        // rather than a silent fall back to HTTP/2, because a caller who set
        // the flag meant it. See patches/UPSTREAM.md.
        if http3_prior_knowledge.unwrap_or(false) {
            return Err(ImpitError::Http3Disabled);
        }

        let url = request.url.to_string();
        let client = &self.base_client;

        let header_map_result: Result<HeaderMap, ImpitError> =
            HttpHeaders::from(request.headers).into();
        let header_map = header_map_result?;

        let method = Method::from_str(&request.method).map_err(|_| {
            ImpitError::InvalidMethod(format!("Invalid HTTP method: {}", request.method))
        })?;

        let max_redirects = match self.config.redirect {
            RedirectBehavior::FollowRedirect(max) => max,
            RedirectBehavior::ManualRedirect => 0,
        };

        let mut prepared = PreparedRequest {
            method: method.clone(),
            url: request.url.clone(),
            headers: header_map,
            body: request.body,
        };

        let primary_result = self.execute_request(client, &mut prepared, timeout).await;

        let response = match primary_result {
            Ok(resp) => resp,
            Err(err) => {
                let primary_error = ImpitError::from(
                    err,
                    Some(ErrorContext {
                        timeout: Some(timeout.unwrap_or(self.config.request_timeout)),
                        max_redirects: Some(max_redirects),
                        method: Some(method.to_string()),
                        protocol: Some(request.url.scheme().to_string()),
                        url: Some(url.clone()),
                    }),
                );

                let fallback_client = self
                    .vanilla_client
                    .as_ref()
                    .filter(|_| primary_error.is_connect_error() && prepared.body.is_sendable());
                let Some(vanilla_client) = fallback_client else {
                    return Err(primary_error);
                };

                debug!(
                    "Primary request to {url} failed with {primary_error}, retrying with vanilla client"
                );
                match self
                    .execute_request(vanilla_client, &mut prepared, timeout)
                    .await
                {
                    Ok(resp) => resp,
                    Err(_) => return Err(primary_error),
                }
            }
        };

        Ok(response)
    }

    async fn make_request(
        &self,
        method: Method,
        url: String,
        body: Option<ImpitBody>,
        options: Option<RequestOptions>,
    ) -> Result<Response, ImpitError> {
        let url = self.parse_url(url)?;
        let request_options = options.unwrap_or_default();

        let headers = request_options.headers;
        let request = self.build_request(method, url, body, headers);

        let timeout = match request_options.timeout {
            None => None,
            // reqwest has no per-request "no timeout" API; overriding with Duration::MAX is the
            // conventional way to disable a timeout without rebuilding the client.
            Some(None) => Some(Duration::MAX),
            Some(Some(d)) => Some(d),
        };
        let http3_prior_knowledge = request_options.http3_prior_knowledge;
        self.send(request, timeout, Some(http3_prior_knowledge))
            .await
    }

    /// Makes a `GET` request to the specified URL.
    ///
    /// The `url` parameter should be a valid URL.
    /// Additional options like `headers`, `timeout` or HTTP/3 usage can be passed via the `RequestOptions` struct.
    ///
    /// If the request is successful, the `reqwest::Response` struct is returned.
    pub async fn get(
        &self,
        url: String,
        body: Option<ImpitBody>,
        options: Option<RequestOptions>,
    ) -> Result<Response, ImpitError> {
        self.make_request(Method::GET, url, body, options).await
    }

    /// Makes a `HEAD` request to the specified URL.
    ///
    /// The `url` parameter should be a valid URL.
    /// Additional options like `headers`, `timeout` or HTTP/3 usage can be passed via the `RequestOptions` struct.
    ///
    /// If the request is successful, the `reqwest::Response` struct is returned.
    pub async fn head(
        &self,
        url: String,
        body: Option<ImpitBody>,
        options: Option<RequestOptions>,
    ) -> Result<Response, ImpitError> {
        self.make_request(Method::HEAD, url, body, options).await
    }

    /// Makes an OPTIONS request to the specified URL.
    ///
    /// The `url` parameter should be a valid URL.
    /// Additional options like `headers`, `timeout` or HTTP/3 usage can be passed via the `RequestOptions` struct.
    ///
    /// If the request is successful, the `reqwest::Response` struct is returned.
    pub async fn options(
        &self,
        url: String,
        body: Option<ImpitBody>,
        options: Option<RequestOptions>,
    ) -> Result<Response, ImpitError> {
        self.make_request(Method::OPTIONS, url, body, options).await
    }

    /// Makes a `TRACE` request to the specified URL.
    ///
    /// The `url` parameter should be a valid URL.
    /// Additional options like `headers`, `timeout` or HTTP/3 usage can be passed via the `RequestOptions` struct.
    ///
    /// If the request is successful, the `reqwest::Response` struct is returned.
    pub async fn trace(
        &self,
        url: String,
        body: Option<ImpitBody>,
        options: Option<RequestOptions>,
    ) -> Result<Response, ImpitError> {
        self.make_request(Method::TRACE, url, body, options).await
    }

    /// Makes a `DELETE` request to the specified URL.
    ///
    /// The `url` parameter should be a valid URL.
    /// Additional options like `headers`, `timeout` or HTTP/3 usage can be passed via the `RequestOptions` struct.
    ///
    /// If the request is successful, the `reqwest::Response` struct is returned.
    pub async fn delete(
        &self,
        url: String,
        body: Option<ImpitBody>,
        options: Option<RequestOptions>,
    ) -> Result<Response, ImpitError> {
        self.make_request(Method::DELETE, url, body, options).await
    }

    /// Makes a `POST` request to the specified URL.
    ///
    /// The `url` parameter should be a valid URL.
    /// Additional options like `headers`, `timeout` or HTTP/3 usage can be passed via the `RequestOptions` struct.
    ///
    /// If the request is successful, the `reqwest::Response` struct is returned.
    pub async fn post(
        &self,
        url: String,
        body: Option<ImpitBody>,
        options: Option<RequestOptions>,
    ) -> Result<Response, ImpitError> {
        self.make_request(Method::POST, url, body, options).await
    }

    /// Makes a `PUT` request to the specified URL.
    ///
    /// The `url` parameter should be a valid URL.
    /// Additional options like `headers`, `timeout` or HTTP/3 usage can be passed via the `RequestOptions` struct.
    ///
    /// If the request is successful, the `reqwest::Response` struct is returned.
    pub async fn put(
        &self,
        url: String,
        body: Option<ImpitBody>,
        options: Option<RequestOptions>,
    ) -> Result<Response, ImpitError> {
        self.make_request(Method::PUT, url, body, options).await
    }

    /// Makes a `PATCH` request to the specified URL.
    ///
    /// The `url` parameter should be a valid URL.
    /// Additional options like `headers`, `timeout` or HTTP/3 usage can be passed via the `RequestOptions` struct.
    ///
    /// If the request is successful, the `reqwest::Response` struct is returned.
    pub async fn patch(
        &self,
        url: String,
        body: Option<ImpitBody>,
        options: Option<RequestOptions>,
    ) -> Result<Response, ImpitError> {
        self.make_request(Method::PATCH, url, body, options).await
    }

    pub fn generate_multipart_boundary(&self) -> String {
        match &self.config.fingerprint {
            Some(fp) => fp.generate_multipart_boundary(),
            None => crate::fingerprint::default_multipart_boundary(),
        }
    }
}
