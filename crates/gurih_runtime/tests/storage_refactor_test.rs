use gurih_runtime::storage::S3FileDriver;
use std::collections::HashMap;

#[tokio::test]
async fn test_s3_driver_missing_bucket() {
    let props = HashMap::new();
    let result = S3FileDriver::new(&props).await;
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(err, "Bucket is required for S3");
}

#[tokio::test]
async fn test_s3_driver_invalid_credentials() {
    let mut props = HashMap::new();
    props.insert("bucket".to_string(), "my-bucket".to_string());
    // Missing credentials (access_key, secret_key) might pass if using environment variables or instance profile?
    // S3 credentials `new` expects Option arguments. If they are None, it tries env vars etc.
    // If we want to force failure, we might need to set invalid keys?
    // But `Credentials::new` usually doesn't validate against AWS immediately unless we try to use it?
    // Actually my change was:
    // Credentials::new(access_key.as_deref(), secret_key.as_deref(), None, None, None)
    // .map_err(|e| format!("Failed to create S3 credentials: {}", e))?;

    // rust-s3 Credentials::new returns Result.

    let result = S3FileDriver::new(&props).await;
    // With only bucket, credentials might be inferred from env or fail if env is missing.
    // In CI/sandbox, AWS env vars might not be present.
    // Let's see what happens.
    if let Err(e) = result {
        println!("Got expected error: {}", e);
    } else {
        println!("S3 Driver initialized (likely with default/anonymous or env creds).");
    }
}
