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
use crate::config::Config;

pub type Req = Request<hyper::body::Incoming>;
pub type Res = Response<Full<Bytes>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logs::init();
    let cfg = Config::get().await?;

    let listener = TcpListener::bind(&cfg.addr_server).await?;
    info!("Running the CF-HUB [{}] on: {}", VERSION, cfg.addr_server);

    let acceptor_cf = cloudflare::TlsAcceptorCF::init()
        .expect("Failed to initialize the Cloudflare TLS!");

    let service_handler = service_fn(move |req| {
        async move {
            let result = service(req).await;

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
                .serve_connection(TokioIo::new(tls_stream), service_handler)
                .await
            {
                error!("[{}] Error serving connection: {:#?}", addr, err);
            }
        });
    }
}

pub async fn service(req: Req) -> Result<Res, AnyError> {
    let host = req
        .uri()
        .host()
        .ok_or_else(|| {
            error!("Missing host in the URL [{}]", req.uri());
            AnyError
        })?;

    let cfg = Config::get().await.map_err(|err| {
        error!("Failed to load config: {}", err);
        err
    })?;

    let host_addr = cfg
        .hosts
        .get(host)
        .ok_or(AnyError)
        .map_err(|err| {
            if (cfg.logs) {
                error!("HOST [{}] not found in the config!", host);
            }
            AnyError::from(err)
        })?;

    if (cfg.logs) {
        info!("Got new request for [{}] redirected to [{}]", host, host_addr);
    }

    let node_stream = TcpStream::connect(host_addr).await.map_err(|err| {
        if (cfg.logs) {
            error!("Failed to connect to the [{}]: {}", host_addr, err);
        }
        err
    })?;

    let io = TokioIo::new(node_stream);
    let (mut sender, conn) = handshake(io)
        .await
        .map_err(|err| {
            if (cfg.logs) {
                error!("Failed the handshake for [{}]: {}", host_addr, err);
            }
            err
        })?;

    let proxy = async move {
        let (res_head, mut res_body_stream) = sender
            .send_request(req)
            .await
            .map_err(|err| {
                if (cfg.logs) {
                    error!("Failed to send the request: {}", err);
                } 
                err
            })?
            .into_parts();

        let mut res_body_buffer = Vec::new();
        while let Some(next) = res_body_stream.frame().await {
            if let Ok(chunk) = next.map_err(|err| {
                if (cfg.logs) {
                    error!("Failed while reading the body: {}", err);
                }
                err
            })?.into_data() {
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
