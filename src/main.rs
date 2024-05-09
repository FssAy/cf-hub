#[macro_use]
extern crate tracing;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde;

mod utils;
mod tls;
mod config;

use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Request, Response};
use hyper::body::Bytes;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use crate::utils::consts::VERSION;
use crate::utils::*;
use tls::*;
use hyper_util::server::conn::auto::Builder;
use hyper_util::rt::TokioExecutor;
use http_body_util::Full;

pub type Req = Request<hyper::body::Incoming>;
pub type Res = Response<Full<Bytes>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logs::init();
    let cfg = config::Config::load().await?;

    let listener = TcpListener::bind(&cfg.addr_server).await?;
    info!("Running the CF-HUB [{}] on: {}", VERSION, cfg.addr_server);

    let acceptor_cf = cloudflare::TlsAcceptorCF::init()
        .expect("Failed to initialize the Cloudflare TLS!");

    loop {
        let (stream, addr) = listener.accept().await?;
        debug!("[{}] new connection", addr);

        let acceptor = acceptor_cf.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(tls_stream) => tls_stream,
                Err(err) => {
                    #[cfg(debug_assertions)]
                    error!("[{}] Failed to perform a TLS handshake: {:#?}", addr, err);

                    // to disable warning on release build
                    drop(err);

                    return;
                }
            };

            if let Err(err) = Builder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(tls_stream), service_fn(|req| async move {
                    service(req, addr).await
                }))
                .await
            {
                error!("[{}] Error serving connection: {:#?}", addr, err);
            }
        });
    }
}

pub async fn service(req: Req, addr: SocketAddr) -> Result<Res, Infallible> {
    Ok(Res::new(
        body!(static b"Hello World")
    ))
}
