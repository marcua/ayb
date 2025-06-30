use crate::error::AybError;
use crate::server::config::AybConfigSnapshots;
use crate::server::snapshots::models::{ListSnapshotResult, Snapshot};
use futures_util::future::join_all;
use log::{debug, error, info, warn};
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::{Bucket, Region};
use std::fs::File;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use zstd::stream::{Decoder, Encoder};

pub struct SnapshotStorage {
    bucket: Bucket,
    path_prefix: String,
}

impl SnapshotStorage {
    pub async fn new(config: &AybConfigSnapshots) -> Result<SnapshotStorage, AybError> {
        info!("Creating SnapshotStorage with config: bucket={}, endpoint_url={:?}, region={:?}, force_path_style={:?}", 
            config.bucket, config.endpoint_url, config.region, config.force_path_style);

        debug!("Creating S3 credentials...");
        let credentials = Credentials::new(
            Some(&config.access_key_id),
            Some(&config.secret_access_key),
            None,
            None,
            None,
        )
        .map_err(|err| {
            error!("Failed to create S3 credentials: {:?}", err);
            AybError::S3ExecutionError {
                message: format!("Failed to create S3 credentials: {:?}", err),
            }
        })?;
        debug!("S3 credentials created successfully");

        let region_str = config.region.clone().unwrap_or("us-east-1".to_string());
        debug!("Using region: {}", region_str);

        let region = if let Some(endpoint_url) = &config.endpoint_url {
            info!("Using custom S3 endpoint: {}", endpoint_url);
            Region::Custom {
                region: region_str.clone(),
                endpoint: endpoint_url.to_string(),
            }
        } else {
            info!("Using AWS region: {}", region_str);
            region_str.parse().map_err(|err| {
                error!("Failed to parse region {}: {:?}", region_str, err);
                AybError::S3ExecutionError {
                    message: format!("Failed to parse region: {}, {:?}", region_str, err),
                }
            })?
        };

        debug!("Creating S3 bucket connection...");
        let mut bucket = Bucket::new(&config.bucket, region, credentials).map_err(|err| {
            error!("Failed to create bucket connection: {:?}", err);
            AybError::S3ExecutionError {
                message: format!("Failed to load bucket: {:?}", err),
            }
        })?;

        let force_path_style = config.force_path_style.unwrap_or(false);
        let mut path_prefix = config.path_prefix.clone();

        if force_path_style {
            info!("Enabling path-style S3 access");
            bucket = bucket.with_path_style();
            path_prefix = format!("{}/{}", &config.bucket, path_prefix);
            debug!("Updated path_prefix for path-style: {}", path_prefix);
        }

        info!(
            "SnapshotStorage created successfully with path_prefix: {}",
            path_prefix
        );
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
        let start_time = Instant::now();
        info!(
            "Starting batch delete of {} snapshots for {}/{}",
            snapshot_ids.len(),
            entity_slug,
            database_slug
        );
        debug!("Snapshots to delete: {:?}", snapshot_ids);

        let delete_futures: Vec<_> = snapshot_ids
            .iter()
            .map(|snapshot_id| {
                let key = self
                    .db_path(entity_slug, database_slug, snapshot_id)
                    .clone();
                let snapshot_id_copy = snapshot_id.clone();

                async move {
                    debug!("Deleting snapshot {} (key: {})", snapshot_id_copy, key);
                    let delete_start = Instant::now();

                    let result = self.bucket.delete_object(&key).await.map_err(|err| {
                        error!(
                            "Failed to delete snapshot {} (key: {}): {:?}",
                            snapshot_id_copy, key, err
                        );
                        AybError::S3ExecutionError {
                            message: format!("Failed to delete snapshot {}: {:?}", key, err),
                        }
                    });

                    match &result {
                        Ok(_) => {
                            debug!(
                                "Successfully deleted snapshot {} in {:?}",
                                snapshot_id_copy,
                                delete_start.elapsed()
                            );
                        }
                        Err(_) => {
                            // Error already logged above
                        }
                    }

                    result
                }
            })
            .collect();

        // Await all delete operations
        debug!("Awaiting {} delete operations", delete_futures.len());
        let results = join_all(delete_futures).await;

        // Handle errors
        let mut error_count = 0;
        for (i, result) in results.into_iter().enumerate() {
            if let Err(ref err) = result {
                error_count += 1;
                error!("Delete operation {} failed: {:?}", i, err);
            }
            result?; // Return the first error, if any
        }

        if error_count == 0 {
            info!(
                "Successfully deleted {} snapshots for {}/{} in {:?}",
                snapshot_ids.len(),
                entity_slug,
                database_slug,
                start_time.elapsed()
            );
        } else {
            error!(
                "Failed to delete {} out of {} snapshots for {}/{}",
                error_count,
                snapshot_ids.len(),
                entity_slug,
                database_slug
            );
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
        let start_time = Instant::now();
        let s3_path = self.db_path(entity_slug, database_slug, snapshot_id);
        let mut snapshot_path = destination_path.to_path_buf();
        snapshot_path.push(database_slug);

        info!(
            "Retrieving snapshot {} for {}/{} from S3 path: {} to local path: {:?}",
            snapshot_id, entity_slug, database_slug, s3_path, snapshot_path
        );

        debug!("Fetching object from S3...");
        let fetch_start = Instant::now();
        let response = self
            .bucket
            .get_object(&s3_path)
            .await
            .map_err(|err| match err {
                S3Error::HttpFailWithBody(status_code, ref body) => {
                    if status_code == 404 && body.contains("<Code>NoSuchKey</Code>") {
                        warn!(
                            "Snapshot {} does not exist at S3 path: {}",
                            snapshot_id, s3_path
                        );
                        return AybError::SnapshotDoesNotExistError;
                    }
                    error!(
                        "HTTP error {} retrieving snapshot {}: {}",
                        status_code, s3_path, body
                    );
                    AybError::S3ExecutionError {
                        message: format!("Failed to retrieve snapshot {}: {:?}", s3_path, err),
                    }
                }
                _ => {
                    error!("S3 error retrieving snapshot {}: {:?}", s3_path, err);
                    AybError::S3ExecutionError {
                        message: format!("Failed to retrieve snapshot {}: {:?}", s3_path, err),
                    }
                }
            })?;

        let response_size = response.bytes().len();
        debug!(
            "Retrieved {} bytes from S3 in {:?}",
            response_size,
            fetch_start.elapsed()
        );

        debug!("Decompressing snapshot data...");
        let decompress_start = Instant::now();
        let body = Cursor::new(response.bytes());
        let mut decoder = Decoder::new(body).map_err(|err| {
            error!("Failed to create zstd decoder: {:?}", err);
            err
        })?;
        let mut decompressed_data = Vec::new();
        io::copy(&mut decoder, &mut decompressed_data).map_err(|err| {
            error!("Failed to decompress snapshot data: {:?}", err);
            err
        })?;

        let decompressed_size = decompressed_data.len();
        debug!(
            "Decompressed {} bytes to {} bytes in {:?}",
            response_size,
            decompressed_size,
            decompress_start.elapsed()
        );

        debug!("Writing decompressed data to file: {:?}", snapshot_path);
        let write_start = Instant::now();
        let mut file = File::create(&snapshot_path).map_err(|err| {
            error!(
                "Failed to create snapshot file {:?}: {:?}",
                snapshot_path, err
            );
            err
        })?;
        file.write_all(&decompressed_data).map_err(|err| {
            error!(
                "Failed to write snapshot data to {:?}: {:?}",
                snapshot_path, err
            );
            err
        })?;
        debug!(
            "Wrote {} bytes to file in {:?}",
            decompressed_size,
            write_start.elapsed()
        );

        info!("Successfully retrieved snapshot {} for {}/{} in {:?} (compressed: {} bytes, decompressed: {} bytes)", 
            snapshot_id, entity_slug, database_slug, start_time.elapsed(), response_size, decompressed_size);

        Ok(())
    }

    pub async fn list_snapshots(
        &self,
        entity_slug: &str,
        database_slug: &str,
    ) -> Result<Vec<ListSnapshotResult>, AybError> {
        let start_time = Instant::now();
        let path = self.db_path(entity_slug, database_slug, "");

        info!(
            "Listing snapshots for {}/{} at S3 path: {}",
            entity_slug, database_slug, path
        );

        debug!("Calling S3 list operation...");
        let list_start = Instant::now();
        let results = self.bucket.list(path.clone(), None).await.map_err(|err| {
            error!("Failed to list snapshots at path {}: {:?}", path, err);
            AybError::S3ExecutionError {
                message: format!("Failed to list snapshots: {:?}", err),
            }
        })?;
        debug!("S3 list operation completed in {:?}", list_start.elapsed());

        let mut snapshots = Vec::new();
        let mut total_objects = 0;
        let mut valid_snapshots = 0;

        for result in results {
            total_objects += result.contents.len();
            debug!(
                "Processing {} objects from S3 list result",
                result.contents.len()
            );

            for object in result.contents {
                let key = object.key.clone();
                debug!(
                    "Processing S3 object: {} (size: {}, modified: {})",
                    key, object.size, object.last_modified
                );

                if let Some(snapshot_id) = key.rsplit('/').next() {
                    if !snapshot_id.is_empty() {
                        debug!("Extracted snapshot ID: {} from key: {}", snapshot_id, key);

                        let parsed_date = object.last_modified.parse().map_err(|err| {
                            error!(
                                "Failed to parse last modified datetime '{}' from object {}: {:?}",
                                object.last_modified, key, err
                            );
                            AybError::S3ExecutionError {
                                message: format!(
                                    "Failed to read last modified datetime from object {}: {:?}",
                                    key, err
                                ),
                            }
                        })?;

                        snapshots.push(ListSnapshotResult {
                            last_modified_at: parsed_date,
                            snapshot_id: snapshot_id.to_string(),
                        });
                        valid_snapshots += 1;
                    } else {
                        debug!("Skipping object with empty snapshot ID: {}", key);
                    }
                } else {
                    debug!("Could not extract snapshot ID from key: {}", key);
                }
            }
        }

        debug!(
            "Processed {} total objects, found {} valid snapshots",
            total_objects, valid_snapshots
        );

        // Return results in descending order.
        debug!(
            "Sorting {} snapshots by last modified date",
            snapshots.len()
        );
        snapshots.sort_by(|a, b| b.last_modified_at.cmp(&a.last_modified_at));

        info!(
            "Successfully listed {} snapshots for {}/{} in {:?}",
            snapshots.len(),
            entity_slug,
            database_slug,
            start_time.elapsed()
        );

        if !snapshots.is_empty() {
            debug!(
                "Most recent snapshot: {} ({})",
                snapshots[0].snapshot_id, snapshots[0].last_modified_at
            );
            if snapshots.len() > 1 {
                debug!(
                    "Oldest snapshot: {} ({})",
                    snapshots[snapshots.len() - 1].snapshot_id,
                    snapshots[snapshots.len() - 1].last_modified_at
                );
            }
        }

        Ok(snapshots)
    }

    pub async fn put(
        &self,
        entity_slug: &str,
        database_slug: &str,
        snapshot: &Snapshot,
        snapshot_path: &PathBuf,
    ) -> Result<(), AybError> {
        let start_time = Instant::now();
        let s3_path = self.db_path(entity_slug, database_slug, &snapshot.snapshot_id);

        info!(
            "Uploading snapshot {} for {}/{} from local path: {:?} to S3 path: {}",
            snapshot.snapshot_id, entity_slug, database_slug, snapshot_path, s3_path
        );

        // Check if source file exists and get its size
        let file_metadata = std::fs::metadata(snapshot_path).map_err(|err| {
            error!(
                "Failed to read metadata for snapshot file {:?}: {:?}",
                snapshot_path, err
            );
            err
        })?;
        let original_size = file_metadata.len();
        debug!("Source file size: {} bytes", original_size);

        debug!("Opening source file for reading...");
        let mut input_file = File::open(snapshot_path).map_err(|err| {
            error!(
                "Failed to open snapshot file {:?}: {:?}",
                snapshot_path, err
            );
            err
        })?;

        debug!("Compressing snapshot data with zstd...");
        let compress_start = Instant::now();
        let mut encoder = Encoder::new(Vec::new(), 0).map_err(|err| {
            error!("Failed to create zstd encoder: {:?}", err);
            err
        })?; // 0 = default compression for zstd

        io::copy(&mut input_file, &mut encoder).map_err(|err| {
            error!("Failed to compress snapshot data: {:?}", err);
            err
        })?;

        let compressed_data = encoder.finish().map_err(|err| {
            error!("Failed to finalize zstd compression: {:?}", err);
            err
        })?;

        let compressed_size = compressed_data.len();
        let compression_ratio = original_size as f64 / compressed_size as f64;
        debug!(
            "Compressed {} bytes to {} bytes (ratio: {:.2}x) in {:?}",
            original_size,
            compressed_size,
            compression_ratio,
            compress_start.elapsed()
        );

        debug!("Uploading {} bytes to S3...", compressed_size);
        let upload_start = Instant::now();
        self.bucket
            .put_object(&s3_path, &compressed_data)
            .await
            .map_err(|err| {
                error!(
                    "Failed to upload snapshot {} to S3 path {}: {:?}",
                    snapshot.snapshot_id, s3_path, err
                );
                AybError::S3ExecutionError {
                    message: format!("Failed to upload snapshot {}: {:?}", s3_path, err),
                }
            })?;
        debug!("Upload completed in {:?}", upload_start.elapsed());

        info!("Successfully uploaded snapshot {} for {}/{} in {:?} (original: {} bytes, compressed: {} bytes, ratio: {:.2}x)",
            snapshot.snapshot_id, entity_slug, database_slug, start_time.elapsed(),
            original_size, compressed_size, compression_ratio);

        Ok(())
    }
}
