pub(crate) type Error = Box<dyn std::error::Error>;
pub(crate) type Result<T> = core::result::Result<T, Error>;
