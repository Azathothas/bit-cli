//! # impit | browser impersonation made simple
//!
//! impit is a `rust` library that allows you to impersonate a browser and make requests to websites. It is built on top of `reqwest`, `rustls` and `tokio` and supports HTTP/1.1, HTTP/2, and HTTP/3.
//!
//! The library provides a simple API for making requests to websites, and it also allows you to customize the request headers, use proxies, custom timeouts and more.
//!
//! ```rust,no_run
//! use impit::{impit::Impit, fingerprint::database as fingerprints};
//! use reqwest::cookie::Jar;
//!
//! #[tokio::main]
//! async fn main() {
//!    let impit = Impit::<Jar>::builder()
//!        .with_fingerprint(fingerprints::firefox_144::fingerprint())
//!        .with_http3()
//!        .build()
//!        .unwrap();
//!
//!    let response = impit.get(String::from("https://example.com"), None, None).await;
//!
//!    match response {
//!        Ok(response) => {
//!            println!("{}", response.text().await.unwrap());
//!        }
//!        Err(e) => {
//!            println!("{:#?}", e);
//!        }
//!    }
//! }
//! ```
//!
//! ### Other projects
//!
//! If you'd prefer to use `impit` from a Node.js application, check out the [`impit-node`](https://github.com/apify/impit/tree/master/impit-node) folder, or download the package from npm:
//! ```bash
//! npm install impit
//! ```
//!
//! ### Usage from Rust
//!
//! Technically speaking, the `impit` project is a somewhat thin wrapper around `reqwest` that provides a more ergonomic API for making requests to websites.
//! The real strength of `impit` is that it uses patched versions of `rustls` and other libraries that allow it to make browser-like requests.
//!
//! Note that if you want to use this library in your rust project, you have to add the following dependencies to your `Cargo.toml` file:
//! ```toml
//! [dependencies]
//! impit = { git="https://github.com/apify/impit.git", branch="master" }
//!
//! [patch.crates-io]
//! rustls = { git="https://github.com/apify/rustls.git" }
//! h2 = { git="https://github.com/apify/h2.git" }
//! ```
//!
//! Without the patched dependencies, the project won't build.

#![deny(unused_crate_dependencies)]
mod http_headers;
mod tls;

/// Main module that contains the `Impit` struct and its methods.
pub mod impit;

/// Customizing request options.
pub mod request;

/// Errors and error handling.
pub mod errors;

/// Browser fingerprint definitions and types.
pub mod fingerprint;

/// Reexports the HTTP/2 extension types a caller needs to configure this
/// client, so a caller does not have to depend on `h2` directly and risk
/// depending on a different `h2` than this one resolves.
///
/// Added by this repository; see patches/UPSTREAM.md.
pub use h2::ext as h2_ext;

/// Reexports reqwest cookie-related types and functions.
/// This module is used to provide a default cookie store for the `Impit` struct.
pub mod cookie {
    pub use reqwest::cookie::*;
}
