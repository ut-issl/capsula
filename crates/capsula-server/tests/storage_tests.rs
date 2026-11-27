//! Integration tests for `capsula_server::storage` against a local directory
//! and an S3-compatible service (versitygw in a container).
#![cfg(test)]

use bytes::Bytes;
use capsula_server::storage::{Storage, StorageError};
use futures::TryStreamExt;
use testcontainers_modules::testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
};
use url::Url;

const S3_PORT: u16 = 7070;
const BUCKET: &str = "capsula-test";
const ACCESS_KEY: &str = "capsula";
const SECRET_KEY: &str = "capsula_dev";

/// A running S3-compatible service with one bucket.
struct S3Service {
    _container: ContainerAsync<GenericImage>,
    endpoint: String,
}

impl S3Service {
    async fn start() -> Self {
        // versitygw serves a directory as buckets, so the bucket is a directory
        // created before the gateway starts.
        let container = GenericImage::new("versity/versitygw", "v1.11.0")
            .with_exposed_port(ContainerPort::Tcp(S3_PORT))
            .with_wait_for(WaitFor::message_on_stdout("listening on"))
            .with_entrypoint("sh")
            .with_cmd([
                "-c",
                &format!("mkdir -p /data/{BUCKET} && exec versitygw posix /data"),
            ])
            .with_env_var("ROOT_ACCESS_KEY", ACCESS_KEY)
            .with_env_var("ROOT_SECRET_KEY", SECRET_KEY)
            .start()
            .await
            .expect("failed to start the S3 container");
        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(S3_PORT).await.unwrap();
        Self {
            _container: container,
            endpoint: format!("http://{host}:{port}"),
        }
    }

    fn storage(&self, url: &str) -> Storage {
        let url: Url = url.parse().unwrap();
        Storage::from_url_with_options(
            &url,
            [
                ("aws_endpoint_url", self.endpoint.as_str()),
                ("aws_access_key_id", ACCESS_KEY),
                ("aws_secret_access_key", SECRET_KEY),
                ("aws_region", "us-east-1"),
                ("aws_allow_http", "true"),
            ],
        )
        .expect("failed to open S3 storage")
    }
}

fn file_storage(dir: &tempfile::TempDir) -> Storage {
    let url = Url::from_directory_path(dir.path()).unwrap();
    Storage::from_url(&url).expect("failed to open file storage")
}

async fn read_all(storage: &Storage, key: &str) -> Option<(u64, Vec<u8>)> {
    let object = storage.get(key).await.unwrap()?;
    let size = object.size;
    let chunks: Vec<Bytes> = object.stream.try_collect().await.unwrap();
    Some((size, chunks.concat()))
}

/// The behavior every backend must show for put / get / exists / delete.
async fn put_get_delete_roundtrip(storage: &Storage) {
    let key = "org-a/sha256/ab/cd/abcd1234";
    assert!(!storage.exists(key).await.unwrap());
    assert!(storage.get(key).await.unwrap().is_none());

    storage
        .put(key, Bytes::from_static(b"hello world"))
        .await
        .unwrap();
    assert!(storage.exists(key).await.unwrap());
    assert_eq!(
        read_all(storage, key).await,
        Some((11, b"hello world".to_vec()))
    );

    // put replaces the previous content
    storage
        .put(key, Bytes::from_static(b"second"))
        .await
        .unwrap();
    assert_eq!(read_all(storage, key).await, Some((6, b"second".to_vec())));

    storage.delete(key).await.unwrap();
    assert!(!storage.exists(key).await.unwrap());
    assert!(storage.get(key).await.unwrap().is_none());

    // deleting again is not an error
    storage.delete(key).await.unwrap();
}

async fn put_stream_roundtrip(storage: &Storage) {
    let key = "org-b/sha256/00/11/stream";
    let chunks = vec![
        Ok(Bytes::from(vec![1u8; 3000])),
        Ok(Bytes::from(vec![2u8; 5000])),
        Ok(Bytes::from(vec![3u8; 100])),
    ];
    let written = storage
        .put_stream(key, futures::stream::iter(chunks))
        .await
        .unwrap();
    assert_eq!(written, 8100);

    let (size, content) = read_all(storage, key).await.unwrap();
    assert_eq!(size, 8100);
    assert_eq!(&content[..3000], &[1u8; 3000]);
    assert_eq!(&content[3000..8000], &[2u8; 5000]);
    assert_eq!(&content[8000..], &[3u8; 100]);
}

async fn put_stream_error_leaves_no_object(storage: &Storage) {
    let key = "org-b/sha256/ff/ff/broken";
    let chunks = vec![
        Ok(Bytes::from_static(b"partial")),
        Err(std::io::Error::other("client went away")),
    ];
    let err = storage
        .put_stream(key, futures::stream::iter(chunks))
        .await
        .unwrap_err();
    assert!(matches!(err, StorageError::Read(_)), "{err}");
    assert!(!storage.exists(key).await.unwrap());
}

async fn rejects_bad_keys(storage: &Storage) {
    for key in ["", "/", "a//b", "a/../b", "./a"] {
        let err = storage.put(key, Bytes::new()).await.unwrap_err();
        assert!(
            matches!(err, StorageError::InvalidKey(_) | StorageError::EmptyKey),
            "{key:?}: {err}"
        );
    }
}

#[tokio::test]
async fn file_backend_put_get_delete() {
    let dir = tempfile::tempdir().unwrap();
    put_get_delete_roundtrip(&file_storage(&dir)).await;
}

#[tokio::test]
async fn file_backend_put_stream() {
    let dir = tempfile::tempdir().unwrap();
    let storage = file_storage(&dir);
    put_stream_roundtrip(&storage).await;
    put_stream_error_leaves_no_object(&storage).await;
}

#[tokio::test]
async fn file_backend_rejects_bad_keys() {
    let dir = tempfile::tempdir().unwrap();
    rejects_bad_keys(&file_storage(&dir)).await;
}

#[tokio::test]
async fn file_backend_stores_objects_under_the_directory() {
    let dir = tempfile::tempdir().unwrap();
    let storage = file_storage(&dir);
    storage
        .put("x/y/z", Bytes::from_static(b"1"))
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(dir.path().join("x").join("y").join("z")).unwrap(),
        b"1"
    );
}

#[tokio::test]
async fn s3_backend_put_get_delete() {
    let s3 = S3Service::start().await;
    put_get_delete_roundtrip(&s3.storage(&format!("s3://{BUCKET}"))).await;
}

#[tokio::test]
async fn s3_backend_put_stream() {
    let s3 = S3Service::start().await;
    let storage = s3.storage(&format!("s3://{BUCKET}"));
    put_stream_roundtrip(&storage).await;
    put_stream_error_leaves_no_object(&storage).await;
}

#[tokio::test]
async fn s3_backend_rejects_bad_keys() {
    let s3 = S3Service::start().await;
    rejects_bad_keys(&s3.storage(&format!("s3://{BUCKET}"))).await;
}

#[tokio::test]
async fn s3_backend_applies_the_url_prefix() {
    let s3 = S3Service::start().await;
    let plain = s3.storage(&format!("s3://{BUCKET}"));
    let prefixed = s3.storage(&format!("s3://{BUCKET}/some/prefix"));

    prefixed.put("k", Bytes::from_static(b"v")).await.unwrap();
    assert!(!plain.exists("k").await.unwrap());
    assert!(plain.exists("some/prefix/k").await.unwrap());
    assert_eq!(read_all(&prefixed, "k").await, Some((1, b"v".to_vec())));
}

#[tokio::test]
async fn s3_backend_reports_missing_bucket() {
    let s3 = S3Service::start().await;
    let storage = s3.storage("s3://no-such-bucket");
    let err = storage.put("k", Bytes::new()).await.unwrap_err();
    assert!(matches!(err, StorageError::ObjectStore(_)), "{err}");
}
