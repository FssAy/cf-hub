lazy_static! {
    pub static ref SERVER_NAME_FULL: String = format!("{}/{}", SERVER_NAME, VERSION);
}

pub static SERVER_NAME: &str = "CF-HUB";
pub static VERSION: &str = env!("CARGO_PKG_VERSION");
