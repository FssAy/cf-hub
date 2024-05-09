pub mod logs;
pub mod consts;

macro_rules! body {
    (static $data:expr) => {
        Full::new(Bytes::from_static($data))
    };

    ($data:expr) => {
        Full::new(Bytes::from($data))
    }
}

pub(crate) use body;
