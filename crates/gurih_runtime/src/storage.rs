use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, config::Region};
use gurih_ir::{StorageDriver, StorageSchema, Symbol};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

#[async_trait]
pub trait FileDriver: Send + Sync {
    async fn put(&self, filename: &str, data: &[u8]) -> Result<String, String>;
    async fn get_url(&self, filename: &str) -> String;
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
    async fn put(&self, filename: &str, data: &[u8]) -> Result<String, String> {
        let path = Path::new(&self.base_path).join(filename);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
        }
        let mut file = tokio::fs::File::create(&path).await.map_err(|e| e.to_string())?;
        file.write_all(data).await.map_err(|e| e.to_string())?;

        Ok(format!("{}/{}", self.base_url, filename))
    }

    async fn get_url(&self, filename: &str) -> String {
        format!("{}/{}", self.base_url, filename)
    }
}

pub struct S3FileDriver {
    client: Client,
    bucket: String,
    public_url: Option<String>,
}

impl S3FileDriver {
    pub async fn new(props: &HashMap<String, String>) -> Self {
        let region = props.get("region").cloned().unwrap_or_else(|| "us-east-1".to_string());
        let bucket = props.get("bucket").cloned().expect("Bucket is required for S3");
        let endpoint = props.get("endpoint").cloned();
        let access_key = props.get("access_key").cloned();
        let secret_key = props.get("secret_key").cloned();
        let public_url = props.get("public_url").cloned();

        let region_provider = RegionProviderChain::first_try(Region::new(region.clone()));

        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest()).region(region_provider);

        if let Some(endpoint) = endpoint {
            config_loader = config_loader.endpoint_url(endpoint);
        }

        if let (Some(ak), Some(sk)) = (access_key, secret_key) {
            let creds = aws_sdk_s3::config::Credentials::new(ak, sk, None, None, "kdl");
            config_loader = config_loader.credentials_provider(creds);
        }

        let config = config_loader.load().await;
        let client = Client::new(&config);

        Self {
            client,
            bucket,
            public_url,
        }
    }
}

#[async_trait]
impl FileDriver for S3FileDriver {
    async fn put(&self, filename: &str, data: &[u8]) -> Result<String, String> {
        let body = aws_sdk_s3::primitives::ByteStream::from(data.to_vec());
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(filename)
            .body(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        Ok(self.get_url(filename).await)
    }

    async fn get_url(&self, filename: &str) -> String {
        if let Some(base) = &self.public_url {
            format!("{}/{}", base.trim_end_matches('/'), filename)
        } else {
            format!("https://{}.s3.amazonaws.com/{}", self.bucket, filename)
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
            let driver: Arc<dyn FileDriver> = match config.driver {
                StorageDriver::Local => {
                    Arc::new(LocalFileDriver::new(config.location.as_deref().unwrap_or("./storage")))
                }
                StorageDriver::S3 => Arc::new(S3FileDriver::new(&config.props).await),
            };
            drivers.insert(name.to_string(), driver);
        }

        Self { drivers }
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn FileDriver>> {
        self.drivers.get(name).cloned()
    }

    pub async fn upload(&self, storage_name: &str, filename: &str, data: &[u8]) -> Result<String, String> {
        if let Some(driver) = self.get(storage_name) {
            driver.put(filename, data).await
        } else {
            Err(format!("Storage '{}' not found", storage_name))
        }
    }
}
