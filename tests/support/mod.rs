use std::{
    ffi::OsStr,
    fs,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A directory owned by one test. Clones keep it alive until the last owner exits.
#[derive(Clone)]
pub struct TestDir(Arc<OwnedPath>);

struct OwnedPath(PathBuf);

impl TestDir {
    pub fn new(prefix: &str) -> Self {
        assert!(prefix.starts_with("hg-") || prefix.starts_with("highgrade-test-"));
        loop {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "{prefix}{}-{nanos}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(Arc::new(OwnedPath(path))),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("isolated test directory: {error}"),
            }
        }
    }
}

impl Deref for TestDir {
    type Target = PathBuf;
    fn deref(&self) -> &PathBuf {
        &self.0.0
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        &self.0.0
    }
}

impl AsRef<OsStr> for TestDir {
    fn as_ref(&self) -> &OsStr {
        self.0.0.as_os_str()
    }
}

impl Drop for OwnedPath {
    fn drop(&mut self) {
        let temp = std::env::temp_dir();
        if self.0.parent() != Some(temp.as_path()) {
            return;
        }
        if let Err(error) = fs::remove_dir_all(&self.0) {
            if error.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "test directory cleanup failed for {}: {error}",
                    self.0.display()
                );
            }
        }
    }
}
