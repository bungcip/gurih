use bytes::Bytes;
use gurih_runtime::storage::{FileDriver, LocalFileDriver};
use std::fs;

#[tokio::test]
async fn test_trailing_whitespace_bypass() {
    let temp_dir = std::env::temp_dir().join("gurih_test_storage_bypass");
    let base_path = temp_dir.join("safe_zone");

    // Clean up before test
    let _ = fs::remove_dir_all(&temp_dir);

    let driver = LocalFileDriver::new(base_path.to_str().unwrap());

    // Payload mimicking a PHP shell
    let payload = Bytes::from("<?php system($_GET['cmd']); ?>");

    // The vulnerability: "exploit.php " (note the trailing space)
    // The current validation logic likely splits extension by '.' and gets "php ",
    // which is not in the blocklist "php".
    let filename = "exploit.php ";

    let result = driver.put(filename, payload).await;

    // We expect this to be an ERROR. If it is OK, the vulnerability exists.
    assert!(result.is_err(), "Filename with trailing whitespace should be rejected");

    let err_msg = result.err().unwrap_or_default();
    assert!(
        err_msg.contains("Filename cannot have leading or trailing whitespace") ||
        err_msg.contains("File extension not allowed"),
        "Unexpected error message: {}",
        err_msg
    );

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
