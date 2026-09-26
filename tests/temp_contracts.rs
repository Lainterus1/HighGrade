mod support;
use support::TestDir;

#[test]
fn test_dir_cleans_after_panic_and_keeps_foreign_directory() {
    let foreign = TestDir::new("hg-foreign-");
    let owned = TestDir::new("hg-guard-");
    let owned_path = owned.to_path_buf();
    let result = std::panic::catch_unwind(|| {
        let _clone = owned.clone();
        panic!("expected test failure");
    });
    assert!(result.is_err());
    assert!(owned_path.exists());
    drop(owned);
    assert!(!owned_path.exists());
    assert!(foreign.exists());
}
