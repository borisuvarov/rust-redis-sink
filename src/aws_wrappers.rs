use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{operation::get_object::GetObjectOutput, Client as S3Client};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

static S3_CLIENT: Lazy<AsyncMutex<Option<Arc<S3Client>>>> = Lazy::new(|| AsyncMutex::new(None));

async fn get_s3_client() -> Arc<S3Client> {
    let mut s3_client_opt = S3_CLIENT.lock().await;
    match &*s3_client_opt {
        Some(s3_client) => Arc::clone(s3_client),
        None => {
            let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
            let config = aws_config::from_env().region(region_provider).load().await;
            let s3_client = Arc::new(S3Client::new(&config));
            *s3_client_opt = Some(Arc::clone(&s3_client));
            s3_client
        }
    }
}

pub async fn get_s3_keys(s3_bucket: String, s3_prefix: String) -> Vec<String> {
    let s3_objects = get_s3_client()
        .await
        .list_objects_v2()
        .bucket(s3_bucket)
        .prefix(s3_prefix)
        .send()
        .await
        .unwrap();
    let mut file_names: Vec<String> = Vec::new();
    for obj in s3_objects.contents().iter() {
        file_names.push(obj.key().unwrap().to_string());
    }
    file_names
}

pub async fn get_s3_object(s3_bucket: String, key: String) -> GetObjectOutput {
    get_s3_client()
        .await
        .get_object()
        .bucket(s3_bucket)
        .key(key)
        .send()
        .await
        .ok()
        .unwrap()
}
