use crate::{
    asynchronous::{
        async_transports::WebsocketSecureTransport as AsyncWebsocketSecureTransport,
        transport::AsyncTransport,
    },
    error::Result,
    transport::Transport,
};
use bytes::Bytes;
use http::HeaderMap;
use native_tls::TlsConnector;
use std::{sync::Arc, borrow::BorrowMut};
use tokio::runtime::{Runtime, self};
use url::Url;

#[derive(Clone)]
pub struct WebsocketSecureTransport {
    runtime: Arc<Runtime>,
    inner: Arc<AsyncWebsocketSecureTransport>,
}

impl WebsocketSecureTransport {
    /// Creates an instance of `WebsocketSecureTransport`.
    pub fn new(
        base_url: Url,
        tls_config: Option<TlsConnector>,
        headers: Option<HeaderMap>,
    ) -> Result<Self> {
        // let runtime = tokio::runtime::Builder::new_current_thread()
        // let runtime = runtime::Runtime::new()?;
            // .enable_all()
            // .worker_threads(100)
            // .build()?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .max_blocking_threads(21000)
            // .worker_threads(1000)
            // .thread_keep_alive(std::time::Duration::from_millis(100))
            .build()?;

        let inner = runtime.block_on(AsyncWebsocketSecureTransport::new(
            base_url, tls_config, headers,
        ))?;

        Ok(WebsocketSecureTransport {
            runtime: Arc::new(runtime),
            inner: Arc::new(inner),
        })
    }

    /// Sends probe packet to ensure connection is valid, then sends upgrade
    /// request
    pub(crate) fn upgrade(&self) -> Result<()> {
        self.runtime.block_on(self.inner.upgrade())
    }
}

impl Transport for WebsocketSecureTransport {
    fn emit(&self, data: Bytes, is_binary_att: bool) -> Result<()> {
        self.runtime.block_on(self.inner.emit(data, is_binary_att))
    }

    fn poll(&self) -> Result<Bytes> {
        self.runtime.block_on(self.inner.poll())
    }

    fn base_url(&self) -> Result<url::Url> {
        self.runtime.block_on(self.inner.base_url())
    }

    fn set_base_url(&self, url: url::Url) -> Result<()> {
        self.runtime.block_on(self.inner.set_base_url(url))
    }
}

impl std::fmt::Debug for WebsocketSecureTransport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "WebsocketSecureTransport(base_url: {:?})",
            self.base_url(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ENGINE_IO_VERSION;
    use std::str::FromStr;
    fn new() -> Result<WebsocketSecureTransport> {
        let url = crate::test::engine_io_server_secure()?.to_string()
            + "engine.io/?EIO="
            + &ENGINE_IO_VERSION.to_string();
        WebsocketSecureTransport::new(
            Url::from_str(&url[..])?,
            Some(crate::test::tls_connector()?),
            None,
        )
    }

    #[test]
    fn websocket_secure_transport_base_url() -> Result<()> {
        let transport = new()?;
        let mut url = crate::test::engine_io_server_secure()?;
        url.set_path("/engine.io/");
        url.query_pairs_mut()
            .append_pair("EIO", &ENGINE_IO_VERSION.to_string())
            .append_pair("transport", "websocket");
        url.set_scheme("wss").unwrap();
        assert_eq!(transport.base_url()?.to_string(), url.to_string());
        transport.set_base_url(reqwest::Url::parse("https://127.0.0.1")?)?;
        assert_eq!(
            transport.base_url()?.to_string(),
            "wss://127.0.0.1/?transport=websocket"
        );
        assert_ne!(transport.base_url()?.to_string(), url.to_string());

        transport.set_base_url(reqwest::Url::parse(
            "http://127.0.0.1/?transport=websocket",
        )?)?;
        assert_eq!(
            transport.base_url()?.to_string(),
            "wss://127.0.0.1/?transport=websocket"
        );
        assert_ne!(transport.base_url()?.to_string(), url.to_string());
        Ok(())
    }

    #[test]
    fn websocket_secure_debug() -> Result<()> {
        let transport = new()?;
        assert_eq!(
            format!("{:?}", transport),
            format!(
                "WebsocketSecureTransport(base_url: {:?})",
                transport.base_url()
            )
        );
        Ok(())
    }
}
