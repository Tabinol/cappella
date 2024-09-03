use std::{
    sync::{Mutex, MutexGuard},
    thread::{self},
    time::{Duration, SystemTime},
};

use super::app_error::AppError;

pub const LOCK_STANDARD_TIMEOUT_DURATION: Duration = Duration::from_secs(5);
const WAIT_SLEEP_DURATION: Duration = Duration::from_millis(10);

pub trait MutexLockTimeout<T>
where
    T: ?Sized,
{
    fn try_lock_timeout(&self, duration: Duration) -> Result<MutexGuard<'_, T>, AppError>;
}

impl<T> MutexLockTimeout<T> for Mutex<T> {
    fn try_lock_timeout(&self, duration: Duration) -> Result<MutexGuard<'_, T>, AppError> {
        if self.is_poisoned() {
            return Err(AppError::new("The lock is poisoned.".to_owned()));
        }

        let start_time = SystemTime::now();

        loop {
            let result = self.try_lock();

            if result.is_ok() {
                return Ok(result?);
            }

            let duration_since_res = SystemTime::now().duration_since(start_time);

            if duration_since_res.is_err() || duration_since_res.unwrap() > duration {
                return Err(AppError::new(
                    "Try lock duration timeout or error in the duration.".to_owned(),
                ));
            }

            thread::sleep(WAIT_SLEEP_DURATION);
        }
    }
}

#[cfg(test)]
mod test {
    use std::{sync::Mutex, time::Duration};

    use crate::local::mutex_lock_timeout::LOCK_STANDARD_TIMEOUT_DURATION;

    use super::MutexLockTimeout;

    #[test]
    fn test_no_timeout() {
        let element = Mutex::new(1_i8);

        let result = *element
            .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
            .unwrap();

        assert_eq!(result, 1);
    }

    #[test]
    fn test_timeout() {
        let element = Mutex::new(1_i8);
        let duration = Duration::from_secs(1);

        let _lock = element.lock().unwrap();

        let result_res = element.try_lock_timeout(duration);

        assert!(result_res.is_err());
    }
}
