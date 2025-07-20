use crate::error::AybError;
use crate::server::config::AybConfigSnapshots;
use crate::server::snapshots::models::{ListSnapshotResult, Snapshot};
use futures_util::future::join_all;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::{Bucket, Region};
use std::fs::File;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use zstd::stream::{Decoder, Encoder};

pub struct SnapshotStorage {
    bucket: Bucket,
    path_prefix: String,
}

impl SnapshotStorage {
    pub async fn new(config: &AybConfigSnapshots) -> Result<SnapshotStorage, AybError> {
        let credentials = Credentials::new(
            Some(&config.access_key_id),
            Some(&config.secret_access_key),
            None,
            None,
            None,
        )
        .map_err(|err| AybError::S3ExecutionError {
            message: format!("Failed to create S3 credentials: {err:?}"),
        })?;

        let region_str = config.region.clone().unwrap_or("us-east-1".to_string());
        let region = if let Some(endpoint_url) = &config.endpoint_url {
            Region::Custom {
                region: region_str,
                endpoint: endpoint_url.to_string(),
            }
        } else {
            region_str
                .parse()
                .map_err(|err| AybError::S3ExecutionError {
                    message: format!("Failed to parse region: {region_str}, {err:?}"),
                })?
        };
        let mut bucket = Bucket::new(&config.bucket, region, credentials).map_err(|err| {
            AybError::S3ExecutionError {
                message: format!("Failed to load bucket: {err:?}"),
            }
        })?;
        let force_path_style = config.force_path_style.unwrap_or(false);
        let mut path_prefix = config.path_prefix.clone();
        if force_path_style {
            bucket = bucket.with_path_style();
            path_prefix = format!("{}/{}", &config.bucket, path_prefix);
        }

        Ok(SnapshotStorage {
            bucket: *bucket,
            path_prefix,
        })
    }

    fn db_path(&self, entity_slug: &str, database_slug: &str, snapshot_id: &str) -> String {
        format!(
            "{}/{}/{}/{}",
            self.path_prefix, entity_slug, database_slug, snapshot_id
        )
    }

    #[allow(clippy::ptr_arg)]
    pub async fn delete_snapshots(
        &self,
        entity_slug: &str,
        database_slug: &str,
        snapshot_ids: &Vec<String>,
    ) -> Result<(), AybError> {
        let delete_futures: Vec<_> = snapshot_ids
            .iter()
            .map(|snapshot_id| {
                let key = self
                    .db_path(entity_slug, database_slug, snapshot_id)
                    .clone();

                async move {
                    self.bucket.delete_object(&key).await.map_err(|err| {
                        AybError::S3ExecutionError {
                            message: format!("Failed to delete snapshot {key}: {err:?}"),
                        }
                    })
                }
            })
            .collect();

        // Await all delete operations
        let results = join_all(delete_futures).await;

        // Handle errors
        for result in results {
            result?; // Return the first error, if any
        }

        Ok(())
    }

    pub async fn retrieve_snapshot(
        &self,
        entity_slug: &str,
        database_slug: &str,
        snapshot_id: &str,
        destination_path: &Path,
    ) -> Result<(), AybError> {
        let s3_path = self.db_path(entity_slug, database_slug, snapshot_id);
        let mut snapshot_path = destination_path.to_path_buf();
        snapshot_path.push(database_slug);

        let response = self
            .bucket
            .get_object(&s3_path)
            .await
            .map_err(|err| match err {
                S3Error::HttpFailWithBody(status_code, ref body) => {
                    if status_code == 404 && body.contains("<Code>NoSuchKey</Code>") {
                        return AybError::SnapshotDoesNotExistError;
                    }
                    AybError::S3ExecutionError {
                        message: format!("Failed to retrieve snapshot {s3_path}: {err:?}"),
                    }
                }
                _ => AybError::S3ExecutionError {
                    message: format!("Failed to retrieve snapshot {s3_path}: {err:?}"),
                },
            })?;

        let body = Cursor::new(response.bytes());
        let mut decoder = Decoder::new(body)?;
        let mut decompressed_data = Vec::new();
        io::copy(&mut decoder, &mut decompressed_data)?;
        let mut file = File::create(snapshot_path)?;
        file.write_all(&decompressed_data)?;

        Ok(())
    }

    pub async fn list_snapshots(
        &self,
        entity_slug: &str,
        database_slug: &str,
    ) -> Result<Vec<ListSnapshotResult>, AybError> {
        let path = self.db_path(entity_slug, database_slug, "");
        let results =
            self.bucket
                .list(path, None)
                .await
                .map_err(|err| AybError::S3ExecutionError {
                    message: format!("Failed to list snapshots: {err:?}"),
                })?;

        let mut snapshots = Vec::new();

        for result in results {
            for object in result.contents {
                let key = object.key;
                if let Some(snapshot_id) = key.rsplit('/').next() {
                    snapshots.push(ListSnapshotResult {
                        last_modified_at: object.last_modified.parse().map_err(|err| {
                            AybError::S3ExecutionError {
                                message: format!(
                                    "Failed to read last modified datetime from object {key}: {err:?}"
                                ),
                            }
                        })?,
                        snapshot_id: snapshot_id.to_string(),
                    });
                }
            }
        }

        // Return results in descending order.
        snapshots.sort_by(|a, b| b.last_modified_at.cmp(&a.last_modified_at));
        Ok(snapshots)
    }

    pub async fn put(
        &self,
        entity_slug: &str,
        database_slug: &str,
        snapshot: &Snapshot,
        snapshot_path: &PathBuf,
    ) -> Result<(), AybError> {
        let path = self.db_path(entity_slug, database_slug, &snapshot.snapshot_id);
        let mut input_file = File::open(snapshot_path)?;
        let mut encoder = Encoder::new(Vec::new(), 0)?; // 0 = default compression for zstd
        io::copy(&mut input_file, &mut encoder)?;
        let compressed_data = encoder.finish()?;

        self.bucket
            .put_object(&path, &compressed_data)
            .await
            .map_err(|err| AybError::S3ExecutionError {
                message: format!("Failed to upload snapshot {path}: {err:?}"),
            })?;

        Ok(())
    }
}
