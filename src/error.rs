
use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Failed(pub String);

impl fmt::Display for Failed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed({})", self.0)
    }
}

impl Error for Failed {}

#[macro_export]
macro_rules! failed {
    ($($arg:expr),*) => {
        return Err(Box::new(crate::error::Failed(format!($($arg),*))))
    };
}