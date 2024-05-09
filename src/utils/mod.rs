pub mod logs;
pub mod consts;
mod any_error;

pub use any_error::*;

macro_rules! body {
    (static $data:expr) => {
        Full::new(Bytes::from_static($data))
    };

    (empty) => {
        Full::new(Bytes::from_static(b""))
    };

    ($data:expr) => {
        Full::new(Bytes::from($data))
    };
}

pub(crate) use body;
