#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum HeapError {
    OutOfMemory,
    InternalError,
}

impl core::fmt::Display for HeapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            HeapError::OutOfMemory => write!(f, "Heap: Out of memory"),
            HeapError::InternalError => write!(f, "Heap: Internal error"),
        }
    }
}
