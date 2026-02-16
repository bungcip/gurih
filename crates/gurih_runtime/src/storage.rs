use async_trait::async_trait;
use bytes::Bytes;
use gurih_ir::{StorageDriver, StorageSchema, Symbol};
use s3::Bucket;
use s3::Region;
use s3::creds::Credentials;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

#[async_trait]
pub trait FileDriver: Send + Sync {
    async fn put(&self, filename: &str, data: Bytes) -> Result<String, String>;
    async fn get_url(&self, filename: &str) -> String;
}

fn validate_filename(filename: &str) -> Result<(), String> {
    if filename.len() > 255 {
        return Err("Filename is too long (max 255 characters)".to_string());
    }

    // Check for control characters
    if filename.chars().any(|c| c.is_control()) {
        return Err("Filename contains invalid characters".to_string());
    }

    // Sentinel: Reject hidden files (e.g. .htaccess) and empty names
    if filename.starts_with('.') || filename.is_empty() {
        return Err("Hidden files or empty filenames are not allowed".to_string());
    }

    // Sentinel: Reject filenames ending with dot (e.g. exploit.php.)
    if filename.ends_with('.') {
        return Err("Filenames cannot end with a dot".to_string());
    }

    let check_path = Path::new(filename);
    if check_path.is_absolute() {
        return Err("Absolute paths are not allowed in storage".to_string());
    }
    for component in check_path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err("Path traversal '..' is not allowed in storage".to_string());
        }
    }

    if let Some(ext) = check_path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        let forbidden = [
            // PHP
            "php", "php3", "php4", "php5", "phtml", "phar", "pht", "pgif", // Scripts & Executables
            "pl", "py", "cgi", "asp", "aspx", "jsp", "jspx", "sh", "bash", "exe", "dll", "bat", "cmd", "vbs", "ps1",
            "wsf", "scr", "msi", "reg", // Web / Java / Misc
            "svg", "html", "htm", "shtml", "xht", "xhtml", "js", "mjs", "class", "jar", "swf", "xml", "xsl",
            "xslt", // XML (Stored XSS risk)
        ];
        if forbidden.contains(&ext_str.as_str()) {
            return Err(format!("File extension not allowed: {}", ext_str));
        }
    }

    Ok(())
}

pub struct LocalFileDriver {
    base_path: String,
    base_url: String,
}

impl LocalFileDriver {
    pub fn new(location: &str) -> Self {
        std::fs::create_dir_all(location).unwrap_or_default();
        Self {
            base_path: location.to_string(),
            base_url: "/storage".to_string(),
        }
    }
}

#[async_trait]
impl FileDriver for LocalFileDriver {
    async fn put(&self, filename: &str, data: Bytes) -> Result<String, String> {
        validate_filename(filename)?;
        let path = Path::new(&self.base_path).join(filename);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        }
        let mut file = tokio::fs::File::create(&path).await.map_err(|e| e.to_string())?;
        file.write_all(&data).await.map_err(|e| e.to_string())?;

        Ok(format!("{}/{}", self.base_url, filename))
    }

    async fn get_url(&self, filename: &str) -> String {
        format!("{}/{}", self.base_url, filename)
    }
}

pub struct S3FileDriver {
    bucket: Box<Bucket>,
    public_url: Option<String>,
}

impl S3FileDriver {
    pub async fn new(props: &HashMap<String, String>) -> Result<Self, String> {
        let region_str = props.get("region").cloned().unwrap_or_else(|| "us-east-1".to_string());
        let bucket_name = props.get("bucket").cloned().ok_or("Bucket is required for S3")?;
        let endpoint = props.get("endpoint").cloned();
        let access_key = props.get("access_key").cloned();
        let secret_key = props.get("secret_key").cloned();
        let public_url = props.get("public_url").cloned();

        let region = if let Some(endpoint) = endpoint {
            Region::Custom {
                region: region_str,
                endpoint,
            }
        } else {
            region_str.parse().unwrap_or(Region::UsEast1)
        };

        let credentials = Credentials::new(access_key.as_deref(), secret_key.as_deref(), None, None, None)
            .map_err(|e| format!("Failed to create S3 credentials: {}", e))?;

        let mut bucket = Bucket::new(&bucket_name, region, credentials)
            .map_err(|e| format!("Failed to create S3 bucket: {}", e))?;

        // If it's a custom endpoint, often we need path style (e.g. Minio)
        if props.contains_key("endpoint") {
            bucket.set_path_style();
        }

        Ok(Self { bucket, public_url })
    }
}

#[async_trait]
impl FileDriver for S3FileDriver {
    async fn put(&self, filename: &str, data: Bytes) -> Result<String, String> {
        validate_filename(filename)?;
        self.bucket
            .put_object(filename, &data)
            .await
            .map_err(|e| e.to_string())?;

        Ok(self.get_url(filename).await)
    }

    async fn get_url(&self, filename: &str) -> String {
        if let Some(base) = &self.public_url {
            format!("{}/{}", base.trim_end_matches('/'), filename)
        } else {
            format!("https://{}.s3.amazonaws.com/{}", self.bucket.name, filename)
        }
    }
}

pub struct StorageEngine {
    drivers: HashMap<String, Arc<dyn FileDriver>>,
}

// ... (existing code for drivers) ...

impl StorageEngine {
    pub async fn new(configs: &HashMap<Symbol, StorageSchema>) -> Self {
        let mut drivers = HashMap::new();

        for (name, config) in configs {
            let driver_res: Result<Arc<dyn FileDriver>, String> = match config.driver {
                StorageDriver::Local => Ok(Arc::new(LocalFileDriver::new(
                    config.location.as_deref().unwrap_or("./storage"),
                ))),
                StorageDriver::S3 => S3FileDriver::new(&config.props)
                    .await
                    .map(|d| Arc::new(d) as Arc<dyn FileDriver>),
            };

            match driver_res {
                Ok(driver) => {
                    drivers.insert(name.to_string(), driver);
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to initialize storage '{}': {}", name, e);
                }
            }
        }

        Self { drivers }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn FileDriver>> {
        self.drivers.get(name).cloned()
    }

    pub async fn upload(&self, storage_name: &str, filename: &str, data: Bytes) -> Result<String, String> {
        if let Some(driver) = self.get(storage_name) {
            driver.put(filename, data).await
        } else {
            Err(format!("Storage '{}' not found", storage_name))
        }
    }
}
