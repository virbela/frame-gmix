pub mod pipeline;
pub mod port_range_manager;
pub mod session_manager;

pub struct MyError {
    pub message: String,
}

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MyError: {}", self.message)
    }
}

impl std::fmt::Debug for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MyError {{ message: {} }}", self.message)
    }
}

impl std::error::Error for MyError {}
