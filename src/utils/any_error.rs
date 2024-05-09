pub struct AnyError;

impl<T: std::error::Error> From<T> for AnyError {
    fn from(_: T) -> Self {
        AnyError
    }
}
