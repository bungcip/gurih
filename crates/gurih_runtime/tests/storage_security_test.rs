use bytes::Bytes;
use gurih_runtime::storage::{FileDriver, LocalFileDriver};
use std::fs;

#[tokio::test]
async fn test_path_traversal_vulnerability() {
    let temp_dir = std::env::temp_dir().join("gurih_test_storage");
    let base_path = temp_dir.join("safe_zone");

    // Clean up before test
    let _ = fs::remove_dir_all(&temp_dir);

    let driver = LocalFileDriver::new(base_path.to_str().unwrap());

    // Attempt path traversal
    let payload = Bytes::from("hacked");
    let filename = "../pwned.txt";

    // With the fix, this should fail
    let result = driver.put(filename, payload).await;

    // Verify the fix
    assert!(result.is_err(), "Path traversal should be rejected");

    let err_msg = result.err().unwrap();
    assert!(
        err_msg.contains("Path traversal '..' is not allowed"),
        "Unexpected error message: {}",
        err_msg
    );

    let pwned_path = temp_dir.join("pwned.txt");
    assert!(!pwned_path.exists(), "File should not be written outside base path");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_dangerous_extension_upload() {
    let temp_dir = std::env::temp_dir().join("gurih_test_storage_ext");
    let base_path = temp_dir.join("safe_zone");
    let _ = fs::remove_dir_all(&temp_dir);

    let driver = LocalFileDriver::new(base_path.to_str().unwrap());

    // Dangerous extensions
    let dangerous_files = vec!["exploit.php", "script.sh", "malware.exe", "xss.html", "xss.svg"];

    for filename in dangerous_files {
        let payload = Bytes::from("malicious content");
        let result = driver.put(filename, payload).await;

        // Assert failure
        assert!(result.is_err(), "Dangerous extension {} should be rejected", filename);
        let err = result.err().unwrap();
        assert!(
            err.contains("File extension not allowed"),
            "Unexpected error for {}: {}",
            filename,
            err
        );
    }

    // Safe extensions
    let safe_files = vec!["image.png", "doc.pdf", "data.json", "readme.txt"];
    for filename in safe_files {
        let payload = Bytes::from("safe content");
        let result = driver.put(filename, payload).await;
        assert!(result.is_ok(), "Safe extension {} should be allowed", filename);
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_enhanced_extension_security() {
    let temp_dir = std::env::temp_dir().join("gurih_test_storage_enhanced");
    let base_path = temp_dir.join("safe_zone");
    let _ = fs::remove_dir_all(&temp_dir);

    let driver = LocalFileDriver::new(base_path.to_str().unwrap());

    // Newly blocked extensions
    let dangerous_files = vec![
        "exploit.phar",
        "template.pht",
        "test.jspx",
        "app.class",
        "library.jar",
        "script.ps1",
        "setup.msi",
        "movie.swf",
        "page.xhtml",
    ];

    for filename in dangerous_files {
        let payload = Bytes::from("malicious content");
        let result = driver.put(filename, payload).await;

        assert!(
            result.is_err(),
            "Enhanced dangerous extension {} should be rejected",
            filename
        );
        let err = result.err().unwrap();
        assert!(
            err.contains("File extension not allowed"),
            "Unexpected error for {}: {}",
            filename,
            err
        );
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_filename_sanitization() {
    let temp_dir = std::env::temp_dir().join("gurih_test_storage_sanitization");
    let base_path = temp_dir.join("safe_zone");
    let _ = fs::remove_dir_all(&temp_dir);

    let driver = LocalFileDriver::new(base_path.to_str().unwrap());
    let payload = Bytes::from("content");

    // 1. Test Control Characters
    let bad_filename = "test\nfile.txt";
    let result = driver.put(bad_filename, payload.clone()).await;
    assert!(result.is_err(), "Filename with newline should be rejected");
    assert_eq!(result.err().unwrap(), "Filename contains invalid characters");

    // 2. Test Max Length
    let long_filename = "a".repeat(256) + ".txt";
    let result = driver.put(&long_filename, payload.clone()).await;
    assert!(result.is_err(), "Filename > 255 chars should be rejected");
    assert_eq!(result.err().unwrap(), "Filename is too long (max 255 characters)");

    let _ = fs::remove_dir_all(&temp_dir);
}
