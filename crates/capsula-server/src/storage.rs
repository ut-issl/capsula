//! Object storage for captured files.
//!
//! [`Storage`] is a thin wrapper around the [`object_store`] crate. The backend
//! is selected by a URL:
//!
//! - `s3://bucket` or `s3://bucket/prefix`: any S3-compatible service. Credentials,
//!   region and endpoint come from the standard `AWS_*` environment variables
//!   (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`,
//!   `AWS_ENDPOINT_URL`, `AWS_SESSION_TOKEN`, ...) or, when no static credentials
//!   are set, from the instance metadata service or a web identity token, as
//!   implemented by [`AmazonS3Builder::from_env`]. `AWS_ALLOW_HTTP=true` permits
//!   plain-HTTP endpoints for local development.
//! - `file:///absolute/path`: a local directory. This backend exists for
//!   development and tests only and is not supported in production.
//!
//! Keys are plain `/`-separated strings; see [`Storage::put`] for the rules.

use bytes::Bytes;
use futures::stream::BoxStream;
use futures::{Stream, StreamExt, TryStreamExt};
use object_store::aws::{AmazonS3Builder, AmazonS3ConfigKey};
use object_store::local::LocalFileSystem;
use object_store::path::Path as ObjectPath;
use object_store::prefix::PrefixStore;
use object_store::{ObjectStore, ObjectStoreExt, ObjectStoreScheme, PutPayload, WriteMultipart};
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

/// Errors returned by [`Storage`].
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The storage URL uses a scheme other than `s3` or `file`.
    #[error(
        "unsupported storage URL `{url}`: use `s3://bucket[/prefix]` or `file:///absolute/path`"
    )]
    UnsupportedScheme { url: Url },
    /// The storage URL has the right scheme but cannot be used.
    #[error("invalid storage URL `{url}`: {reason}")]
    InvalidUrl { url: Url, reason: String },
    /// A storage option has a key the backend does not know.
    #[error("unknown storage option `{key}`")]
    UnknownOption { key: String },
    /// The local storage directory could not be created.
    #[error("failed to create storage directory `{}`: {source}", path.display())]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// An object key is not acceptable (empty, or containing `.`/`..`/empty segments).
    #[error("invalid object key: {0}")]
    InvalidKey(#[from] object_store::path::Error),
    /// An object key is empty.
    #[error("invalid object key: key is empty")]
    EmptyKey,
    /// The underlying object store reported an error.
    #[error(transparent)]
    ObjectStore(#[from] object_store::Error),
    /// Reading the data to upload failed.
    #[error("failed to read upload data: {0}")]
    Read(#[from] std::io::Error),
}

/// An object fetched from storage: its size and a stream of its content.
pub struct StoredObject {
    /// Size of the object in bytes.
    pub size: u64,
    /// The object content.
    pub stream: BoxStream<'static, Result<Bytes, StorageError>>,
}

impl std::fmt::Debug for StoredObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredObject")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// Object storage selected by URL. Cheap to clone; clones share the same backend.
#[derive(Clone)]
pub struct Storage {
    store: Arc<dyn ObjectStore>,
    url: Url,
}

impl std::fmt::Debug for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Storage")
            .field("url", &self.url.as_str())
            .finish_non_exhaustive()
    }
}

impl Storage {
    /// Open the storage named by `url`.
    ///
    /// S3 settings come from the environment (see the module documentation).
    /// A `file://` directory is created when it does not exist.
    pub fn from_url(url: &Url) -> Result<Self, StorageError> {
        Self::from_url_with_options(url, std::iter::empty::<(&str, &str)>())
    }

    /// Open the storage named by `url`, applying `options` on top of the environment.
    ///
    /// Option keys are the [`AmazonS3ConfigKey`] names, for example
    /// `aws_endpoint_url`, `aws_access_key_id`, `aws_secret_access_key`,
    /// `aws_region` or `aws_allow_http`. Options are only meaningful for `s3://`
    /// URLs and are rejected for `file://` URLs.
    pub fn from_url_with_options<I, K, V>(url: &Url, options: I) -> Result<Self, StorageError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        let (scheme, prefix) = ObjectStoreScheme::parse(url)
            .map_err(|_| StorageError::UnsupportedScheme { url: url.clone() })?;

        let store: Arc<dyn ObjectStore> = match scheme {
            ObjectStoreScheme::Local => {
                if let Some((key, _)) = options.into_iter().next() {
                    return Err(StorageError::InvalidUrl {
                        url: url.clone(),
                        reason: format!(
                            "option `{}` is not applicable to file:// storage",
                            key.as_ref()
                        ),
                    });
                }
                let path = url.to_file_path().map_err(|()| StorageError::InvalidUrl {
                    url: url.clone(),
                    reason: "file:// URLs must name an absolute local path".to_owned(),
                })?;
                std::fs::create_dir_all(&path).map_err(|source| StorageError::CreateDirectory {
                    path: path.clone(),
                    source,
                })?;
                Arc::new(LocalFileSystem::new_with_prefix(&path)?)
            }
            ObjectStoreScheme::AmazonS3 => {
                let mut builder = AmazonS3Builder::from_env().with_url(url.as_str());
                for (key, value) in options {
                    let config_key: AmazonS3ConfigKey =
                        key.as_ref()
                            .parse()
                            .map_err(|_| StorageError::UnknownOption {
                                key: key.as_ref().to_owned(),
                            })?;
                    builder = builder.with_config(config_key, value);
                }
                let s3 = builder.build()?;
                if prefix.as_ref().is_empty() {
                    Arc::new(s3)
                } else {
                    Arc::new(PrefixStore::new(s3, prefix))
                }
            }
            _ => return Err(StorageError::UnsupportedScheme { url: url.clone() }),
        };

        Ok(Self {
            store,
            url: url.clone(),
        })
    }

    /// The URL this storage was opened with.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }

    /// Store `data` under `key`, replacing any existing object.
    ///
    /// A key is a `/`-separated path such as `org/sha256/ab/cd/abcd...`. Leading
    /// and trailing slashes are ignored; empty, `.` and `..` segments are rejected.
    pub async fn put(&self, key: &str, data: impl Into<PutPayload>) -> Result<(), StorageError> {
        let location = parse_key(key)?;
        self.store.put(&location, data.into()).await?;
        Ok(())
    }

    /// Store the content of `stream` under `key` without buffering it in memory.
    ///
    /// Returns the number of bytes written. The upload is aborted when the
    /// stream yields an error, and no object is left behind.
    pub async fn put_stream<S>(&self, key: &str, stream: S) -> Result<u64, StorageError>
    where
        S: Stream<Item = Result<Bytes, std::io::Error>>,
    {
        let location = parse_key(key)?;
        let upload = self.store.put_multipart(&location).await?;
        let mut writer = WriteMultipart::new(upload);
        let mut written: u64 = 0;
        let mut stream = std::pin::pin!(stream);
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(e) => {
                    // Best effort: the caller already gets the read error.
                    let _ = writer.abort().await;
                    return Err(StorageError::Read(e));
                }
            };
            written += chunk.len() as u64;
            if let Err(e) = writer.wait_for_capacity(MAX_CONCURRENT_PARTS).await {
                let _ = writer.abort().await;
                return Err(e.into());
            }
            writer.put(chunk);
        }
        writer.finish().await?;
        Ok(written)
    }

    /// Fetch the object stored under `key`, or `None` when there is none.
    pub async fn get(&self, key: &str) -> Result<Option<StoredObject>, StorageError> {
        let location = parse_key(key)?;
        match self.store.get(&location).await {
            Ok(result) => Ok(Some(StoredObject {
                size: result.meta.size,
                stream: result.into_stream().map_err(StorageError::from).boxed(),
            })),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete the object stored under `key`. Deleting a missing object is not an error.
    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let location = parse_key(key)?;
        match self.store.delete(&location).await {
            Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// Whether an object is stored under `key`.
    pub async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let location = parse_key(key)?;
        match self.store.head(&location).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

/// Upper bound on in-flight parts of a streaming upload; bounds memory use.
const MAX_CONCURRENT_PARTS: usize = 4;

/// Validate an object key. `Path::parse` accepts an empty key (the store root),
/// which must never be a target here.
fn parse_key(key: &str) -> Result<ObjectPath, StorageError> {
    let location = ObjectPath::parse(key)?;
    if location.as_ref().is_empty() {
        return Err(StorageError::EmptyKey);
    }
    Ok(location)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_schemes() {
        for url in ["memory:///", "gs://bucket/x", "https://example.com/bucket"] {
            let url: Url = url.parse().unwrap();
            assert!(
                matches!(
                    Storage::from_url(&url),
                    Err(StorageError::UnsupportedScheme { .. })
                ),
                "{url} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_options_for_file_urls() {
        let dir = tempfile::tempdir().unwrap();
        let url = Url::from_directory_path(dir.path()).unwrap();
        let err = Storage::from_url_with_options(&url, [("aws_region", "x")]).unwrap_err();
        assert!(matches!(err, StorageError::InvalidUrl { .. }), "{err}");
    }

    #[test]
    fn rejects_unknown_s3_options() {
        let url: Url = "s3://bucket".parse().unwrap();
        let err = Storage::from_url_with_options(&url, [("no_such_option", "x")]).unwrap_err();
        assert!(matches!(err, StorageError::UnknownOption { .. }), "{err}");
    }

    #[test]
    fn creates_the_local_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        let url = Url::from_directory_path(&nested).unwrap();
        let storage = Storage::from_url(&url).unwrap();
        assert!(nested.is_dir());
        assert_eq!(storage.url(), &url);
    }
}
