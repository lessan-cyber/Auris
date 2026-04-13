use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::info;
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(100)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn check_connection(pool: &PgPool) -> Result<()> {
    sqlx::query("SELECT 1").execute(pool).await?;
    tracing::info!("Database connection is alive");
    Ok(())
}

pub struct S3Client {
    pub client: Client,
    pub bucket_name: String,
    pub max_file_size: usize,
}

impl S3Client {
    pub async fn new(settings: &crate::settings::Settings) -> Result<Self> {
        let credentials = Credentials::new(
            &settings.s3_access_key,
            &settings.s3_secret_key,
            None,
            None,
            "static",
        );
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(aws_config::Region::new("us-east-1"))
            .endpoint_url(&settings.s3_endpoint)
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
            .force_path_style(true)
            .build();

        let client = Client::from_conf(s3_config);
        Ok(Self {
            client,
            bucket_name: settings.s3_bucket_name.clone(),
            max_file_size: settings.max_file_size,
        })
    }
    pub async fn list_buckets(&self) -> Result<()> {
        let response = self.client.list_buckets().send().await?;
        let buckets: Vec<String> = response
            .buckets()
            .iter()
            .filter_map(|b| b.name().map(String::from))
            .collect();
        info!("S3 is up and running. Bucket list: {:?}", &buckets);
        if !buckets.contains(&self.bucket_name) {
            tracing::warn!("Bucket {} not found, creating it", self.bucket_name);
            self.client
                .create_bucket()
                .bucket(&self.bucket_name)
                .send()
                .await?;
        }
        Ok(())
    }
    pub async fn upload_file(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let size = data.len();
        let body = ByteStream::from(data);

        tracing::info!(
            "Uploading file to S3: bucket={}, key={}, size={} bytes",
            self.bucket_name,
            key,
            size
        );

        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .body(body)
            .content_type("application/octet-stream") // Explicit content type
            .send()
            .await
            .map_err(|e| {
                tracing::error!("S3 upload error: {}", e);
                anyhow::anyhow!("S3 upload failed: {}", e)
            })?;

        tracing::info!("S3 upload successful");
        Ok(())
    }
    pub async fn delete_file(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("S3 delete error: {}", e);
                anyhow::anyhow!("S3 delete failed: {}", e)
            })?;
        Ok(())
    }
    pub async fn get_file(&self, key: &str) -> Result<String> {
        tracing::info!("Generating presigned URL for key: {}", key);

        // Create a presigned GET request that expires in 2 days (48 hours)
        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .presigned(
                PresigningConfig::builder()
                    .expires_in(std::time::Duration::from_secs(60 * 60 * 48)) // 48 hours = 2 days
                    .build()?,
            )
            .await?;

        let presigned_url = presigned_request.uri().to_string();
        tracing::info!("Generated presigned URL: {}", presigned_url);

        Ok(presigned_url)
    }
    pub async fn download_file(&self, key: &str) -> Result<Vec<u8>> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await?;

        // Fail closed if content length is unknown to prevent OOM
        let content_length = resp.content_length().ok_or_else(|| {
            anyhow::anyhow!("Cannot download file: content length unknown (potential OOM risk)")
        })?;

        if content_length as usize > self.max_file_size {
            return Err(anyhow::anyhow!(
                "File too large: {} bytes (max: {})",
                content_length,
                self.max_file_size
            ));
        }

        let data = resp.body.collect().await?.to_vec();
        Ok(data)
    }
}
