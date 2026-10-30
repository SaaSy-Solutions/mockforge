use std::path::{Path, PathBuf};
use tokio::sync::{Mutex, MutexGuard};

static CWD_MUTEX: Mutex<()> = Mutex::const_new(());

pub(crate) struct TestCwd {
    previous: PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl TestCwd {
    pub(crate) async fn enter(directory: &Path) -> Self {
        let lock = CWD_MUTEX.lock().await;
        let previous = std::env::current_dir().expect("read original test directory");
        std::env::set_current_dir(directory).expect("enter test directory");
        Self {
            previous,
            _lock: lock,
        }
    }
}

impl Drop for TestCwd {
    fn drop(&mut self) {
        if let Err(error) = std::env::set_current_dir(&self.previous) {
            if std::thread::panicking() {
                eprintln!("Failed to restore test directory: {error}");
            } else {
                panic!("Failed to restore test directory: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::{poll_fn, Future};
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::task::Poll;
    use tempfile::TempDir;

    #[tokio::test]
    async fn independent_directory_changes_wait_for_restoration() {
        let first_directory = TempDir::new().unwrap();
        let second_directory = TempDir::new().unwrap();
        let first = TestCwd::enter(first_directory.path()).await;
        let original = first.previous.clone();
        let mut second = Box::pin(TestCwd::enter(second_directory.path()));

        let outcome = poll_fn(|context| Poll::Ready(second.as_mut().poll(context))).await;
        assert!(outcome.is_pending());
        assert_eq!(std::env::current_dir().unwrap(), first_directory.path());

        drop(first);
        let second = second.await;
        assert_eq!(std::env::current_dir().unwrap(), second_directory.path());
        drop(second);

        let _lock = CWD_MUTEX.lock().await;
        assert_eq!(std::env::current_dir().unwrap(), original);
    }

    #[tokio::test]
    async fn panic_restores_directory_and_releases_the_lock() {
        let directory = TempDir::new().unwrap();
        let scope = TestCwd::enter(directory.path()).await;
        let original = scope.previous.clone();

        let result = catch_unwind(AssertUnwindSafe(move || {
            let _scope = scope;
            panic!("exercise test cleanup");
        }));
        assert!(result.is_err());

        let _lock = CWD_MUTEX.lock().await;
        assert_eq!(std::env::current_dir().unwrap(), original);
    }
}
