use std::time::Duration;

use parking_lot::{Mutex, MutexGuard};

use super::app_error::AppError;

const LOCK_DEFAULT_TIMEOUT_DURATION: Duration = Duration::from_secs(5);
const TIMEOUT_ERROR_MESSAGE: &str = "Lock timeout.";

pub trait MutexLockTimeout<T> {
    fn try_lock_for(&self, duration: Duration) -> Result<MutexGuard<T>, AppError>;
    fn try_lock_default_duration(&self) -> Result<MutexGuard<T>, AppError>;
}

impl<T> MutexLockTimeout<T> for Mutex<T> {
    fn try_lock_default_duration(&self) -> Result<MutexGuard<T>, AppError> {
        MutexLockTimeout::try_lock_for(self, LOCK_DEFAULT_TIMEOUT_DURATION)
    }

    fn try_lock_for(&self, duration: Duration) -> Result<MutexGuard<T>, AppError> {
        self.try_lock_for(duration)
            .ok_or_else(|| AppError::new(TIMEOUT_ERROR_MESSAGE.to_owned()))
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use parking_lot::Mutex;

    use crate::local::mutex_lock_timeout::MutexLockTimeout;

    #[test]
    fn test_no_timeout() {
        let element = Mutex::new(1_i8);

        let result = *element.try_lock_default_duration().unwrap();

        assert_eq!(result, 1);
    }

    #[test]
    fn test_timeout() {
        let element = Mutex::new(1_i8);
        let duration = Duration::from_secs(1);

        let _lock = element.lock();

        let result_res = MutexLockTimeout::try_lock_for(&element, duration);

        assert!(result_res.is_err());
    }
}
