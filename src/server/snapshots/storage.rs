use crate::error::AybError;
use crate::server::config::AybConfigSnapshots;
use crate::server::snapshots::models::{ListSnapshotResult, Snapshot};
use aws_config::meta::region::RegionProviderChain;
use aws_credential_types::Credentials;
use aws_sdk_s3;
use aws_smithy_types_convert::date_time::DateTimeExt;
use aws_types::region::Region;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use zstd::stream::{Decoder, Encoder};

pub struct SnapshotStorage {
    bucket: String,
    client: aws_sdk_s3::Client,
    force_path_style: bool,
    path_prefix: String,
}

impl SnapshotStorage {
    pub async fn new(config: &AybConfigSnapshots) -> Result<SnapshotStorage, AybError> {
        let mut connection_config = aws_config::from_env().credentials_provider(
            Credentials::from_keys(&config.access_key_id, &config.secret_access_key, None),
        );
        if config.endpoint_url.is_some() {
            connection_config =
                connection_config.endpoint_url(config.endpoint_url.as_ref().unwrap());
        }

        let region = Region::new(config.region.clone().unwrap_or("us-east-1".to_string()));
        let region_provider = RegionProviderChain::first_try(region).or_default_provider();
        connection_config = connection_config.region(region_provider);

        let force_path_style = config.force_path_style.unwrap_or(false);
        let s3_config = aws_sdk_s3::config::Builder::from(&connection_config.load().await)
            .force_path_style(force_path_style)
            .build();
        Ok(SnapshotStorage {
            bucket: config.bucket.clone(),
            client: aws_sdk_s3::Client::from_conf(s3_config),
            force_path_style,
            path_prefix: config.path_prefix.to_string(),
        })
    }

    fn db_path(&self, entity_slug: &str, database_slug: &str, snapshot_id: &str) -> String {
        // Include bucket details in path only if `force_path_style` is `true`.
        let bucket = if self.force_path_style {
            format!("{}/", self.bucket)
        } else {
            "".to_string()
        };
        format!(
            "{}{}/{}/{}/{}",
            bucket, self.path_prefix, entity_slug, database_slug, snapshot_id
        )
    }

    pub async fn delete_snapshots(
        &self,
        entity_slug: &str,
        database_slug: &str,
        snapshot_ids: &Vec<String>,
    ) -> Result<(), AybError> {
        let mut delete_objects: Vec<aws_sdk_s3::types::ObjectIdentifier> = vec![];
        for snapshot_id in snapshot_ids {
            let obj_id = aws_sdk_s3::types::ObjectIdentifier::builder()
                .set_key(Some(self.db_path(entity_slug, database_slug, snapshot_id)))
                .build()
                .map_err(|err| AybError::S3ExecutionError {
                    message: format!(
                        "Unable to create object identifier for deletion of {}/{}/{}: {:?}",
                        entity_slug, database_slug, snapshot_id, err
                    ),
                })?;
            delete_objects.push(obj_id);
        }

        if !delete_objects.is_empty() {
            self.client
                .delete_objects()
                .bucket(&self.bucket)
                .delete(
                    aws_sdk_s3::types::Delete::builder()
                        .set_objects(Some(delete_objects))
                        .build()
                        .map_err(|err| AybError::S3ExecutionError {
                            message: format!(
                                "Unable to create deletion builder for {}/{}: {:?}",
                                entity_slug, database_slug, err
                            ),
                        })?,
                )
                .send()
                .await
                .map_err(|err| AybError::S3ExecutionError {
                    message: format!(
                        "Unable to delete snapshots as listed in {}/{}: {:?}",
                        entity_slug, database_slug, err
                    ),
                })?;
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
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(s3_path.clone())
            .send()
            .await
            .map_err(|err| {
                let s3_error = aws_sdk_s3::Error::from(err);
                if let aws_sdk_s3::Error::NoSuchKey(_err) = s3_error {
                    return AybError::SnapshotDoesNotExistError;
                }
                AybError::S3ExecutionError {
                    message: format!(
                        "Unable to retrieve snapshot in S3 at {}: {:?}",
                        s3_path, s3_error
                    ),
                }
            })?;
        let stream = response
            .body
            .collect()
            .await
            .map_err(|err| AybError::S3ExecutionError {
                message: format!(
                    "Unable to stream snapshot retrieval from S3 at {}: {:?}",
                    s3_path, err
                ),
            })?;
        let data = stream.into_bytes();
        let mut decoder = Decoder::new(&data[..])?;
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
        let mut response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(path.clone())
            .into_paginator()
            .send();
        let mut results = Vec::<ListSnapshotResult>::new();

        while let Some(result) = response.next().await {
            match result {
                Ok(output) => {
                    for object in output.contents() {
                        let key = object
                            .key
                            .as_ref()
                            .ok_or_else(|| AybError::S3ExecutionError {
                                message: format!("Unable to read key from object: {:?}", object),
                            })?
                            .clone();
                        let snapshot_id = key
                            .rsplit_once('/')
                            .ok_or_else(|| AybError::S3ExecutionError {
                                message: format!(
                                    "Unexpected key path {} on object: {:?}",
                                    key, object
                                ),
                            })?
                            .1;
                        results.push(ListSnapshotResult {
                            last_modified_at: object
                                .last_modified
                                .map(|t| t.to_chrono_utc())
                                .ok_or_else(|| AybError::S3ExecutionError {
                                    message: format!(
                                        "Unable to read last modified datetime from object: {:?}",
                                        object
                                    ),
                                })??,
                            snapshot_id: snapshot_id.to_string(),
                        });
                    }
                }
                Err(err) => {
                    return Err(AybError::S3ExecutionError {
                        message: format!("Unable to list S3 path: {} ({:?})", path, err),
                    });
                }
            }
        }

        // Return results in descending order.
        results.sort_by(|a, b| b.last_modified_at.cmp(&a.last_modified_at));
        Ok(results)
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
        let body = aws_sdk_s3::primitives::ByteStream::from(encoder.finish()?);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(path.clone())
            .body(body)
            .set_metadata(Some(snapshot.to_header_map()?))
            .send()
            .await
            .map_err(|err| AybError::S3ExecutionError {
                message: format!("Unable to put snapshot in S3 at {}: {:?}", path, err),
            })?;
        Ok(())
    }
}
