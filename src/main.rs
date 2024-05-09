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
use std::sync::Arc;
use hyper::{Request, Response, StatusCode};
use hyper::body::Bytes;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use crate::utils::consts::VERSION;
use crate::utils::*;
use tls::*;
use hyper_util::server::conn::auto::Builder;
use hyper_util::rt::TokioExecutor;
use http_body_util::{BodyExt, Full};
use hyper::client::conn::http1::handshake;
use hyper::header::{HeaderValue, HOST};
use crate::config::{Config, ConfigData};

pub type Req = Request<hyper::body::Incoming>;
pub type Res = Response<Full<Bytes>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logs::init();
    let cfg = ConfigData::load().await?;

    let listener = TcpListener::bind(&cfg.addr_server).await?;
    info!("Running the CF-HUB [{}] on: {}", VERSION, cfg.addr_server);

    let acceptor_cf = cloudflare::TlsAcceptorCF::init()
        .expect("Failed to initialize the Cloudflare TLS!");

    loop {
        let (stream, addr) = listener.accept().await?;
        debug!("[{}] new connection", addr);

        let acceptor = acceptor_cf.clone();

        let cfg = cfg.clone();
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

            let abc = Arc::clone(&cfg);
            let service = service_fn(move |req| {
                let def = Arc::clone(&abc);
                async move {
                    let result = service(req, def).await;

                    if let Err(_) = &result {
                        let res = Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .body(body!(empty))
                            .unwrap();

                        return Ok(res);
                    }

                    result.map_err(|_| unsafe {
                        std::mem::zeroed::<Infallible>()
                    })
                }
            });

            if let Err(err) = Builder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(tls_stream), service)
                .await
            {
                error!("[{}] Error serving connection: {:#?}", addr, err);
            }
        });
    }
}

pub async fn service(mut req: Req, cfg: Config) -> Result<Res, AnyError> {
    let headers = req.headers_mut();

    let node = headers
        .get("node")
        .ok_or(AnyError)?
        .to_str()?;

    let node_addr = cfg
        .nodes
        .get(node)
        .ok_or(AnyError)?;

    let node_stream = TcpStream::connect(node_addr).await?;
    let io = TokioIo::new(node_stream);
    let (mut sender, conn) = handshake(io).await?;

    let proxy = async move {
        let (res_head, mut res_body_stream) = sender.send_request(req).await?.into_parts();

        let mut res_body_buffer = Vec::new();
        while let Some(next) = res_body_stream.frame().await {
            if let Ok(chunk) = next?.into_data() {
                res_body_buffer.extend(chunk);
            }
        }

        let res = Res::from_parts(res_head, body!(res_body_buffer));
        Ok(res)
    };

    tokio::select! {
        result = proxy => result,
        _ = conn => Err(AnyError),
    }
}
