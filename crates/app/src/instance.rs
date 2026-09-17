use anyhow::{Context, Result};
use std::{
    fs::{File, OpenOptions, TryLockError},
    path::Path,
};

/// The operating system releases this lock if the companion closes or crashes.
pub fn acquire(path: &Path) -> Result<Option<File>> {
    std::fs::create_dir_all(path.parent().context("Missing settings folder")?)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn duplicate_is_blocked_and_crash_lock_is_released() {
        let path =
            std::env::temp_dir().join(format!("scopobot-instance-{}.lock", std::process::id()));
        let first = super::acquire(&path).unwrap().unwrap();
        assert!(super::acquire(&path).unwrap().is_none());
        drop(first);
        assert!(super::acquire(&path).unwrap().is_some());
        std::fs::remove_file(path).unwrap();
    }
}
