use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use tokio::fs::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::OnceCell;
use super::*;

static PATH: &str = "cf-hub-cfg.json";

static CONFIG: OnceCell<Config> = OnceCell::const_new();

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub addr_server: SocketAddr,
    pub hosts: HashMap<String, SocketAddr>
}

impl Default for Config {
    fn default() -> Self {
        Self {
            addr_server: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                80,
            ),
            hosts: {
                let mut map = HashMap::new();
                map.insert(
                    "<YOUR_HOST>".to_string(),
                    SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::LOCALHOST),
                        1234,
                    ),
                );
                map
            },
        }
    }
}

impl Config {
    pub async fn get() -> std::io::Result<&'static Config> {
        CONFIG.get_or_try_init(Self::load).await
    }

    async fn load() -> std::io::Result<Config> {
        let config_path = Path::new(PATH);

        let config = if !config_path.is_file() {
            warn!("Missing config file [{}], creating a default one.", PATH);
            let config = Self::default();

            if let Some(dir_path) = config_path.parent() {
                let _ = create_dir_all(dir_path).await;
            }

            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(config_path)
                .await?;

            file.write_all(
                serde_json::to_string_pretty(&config)
                    .unwrap()
                    .as_bytes()
            ).await?;

            config
        } else {
            let mut buffer = String::new();

            OpenOptions::new()
                .read(true)
                .open(config_path)
                .await?
                .read_to_string(&mut buffer)
                .await?;

            match serde_json::from_str::<Self>(buffer.as_str()) {
                Ok(config) => config,
                Err(err) => return Err(
                    Error::new(
                        ErrorKind::Other,
                        format!("Invalid JSON structure for the config file [{}]: {}", PATH, err)
                    )
                ),
            }
        };

        Ok(config)
    }
}
