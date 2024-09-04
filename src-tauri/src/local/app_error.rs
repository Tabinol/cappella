use std::{
    ffi::NulError,
    fmt::Display,
    num::TryFromIntError,
    str::Utf8Error,
    sync::{mpsc::RecvTimeoutError, TryLockError},
    time::SystemTimeError,
};

#[derive(Clone, Debug)]
pub struct AppError {
    error_message: String,
}

impl AppError {
    pub fn new(error_message: String) -> Self {
        Self { error_message }
    }
}

impl<Guard> From<TryLockError<Guard>> for AppError {
    fn from(value: TryLockError<Guard>) -> Self {
        AppError {
            error_message: value.to_string(),
        }
    }
}

impl From<NulError> for AppError {
    fn from(value: NulError) -> Self {
        AppError {
            error_message: value.to_string(),
        }
    }
}

impl From<RecvTimeoutError> for AppError {
    fn from(value: RecvTimeoutError) -> Self {
        AppError {
            error_message: value.to_string(),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError {
            error_message: value.to_string(),
        }
    }
}

impl From<SystemTimeError> for AppError {
    fn from(value: SystemTimeError) -> Self {
        AppError {
            error_message: value.to_string(),
        }
    }
}

impl From<TryFromIntError> for AppError {
    fn from(value: TryFromIntError) -> Self {
        AppError {
            error_message: value.to_string(),
        }
    }
}

impl From<Utf8Error> for AppError {
    fn from(value: Utf8Error) -> Self {
        AppError {
            error_message: value.to_string(),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error_message: {}", self.error_message)
    }
}
