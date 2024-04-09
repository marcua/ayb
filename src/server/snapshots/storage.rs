use crate::error::AybError;
use crate::server::config::AybConfigSnapshots;
use crate::server::snapshots::models::{ListSnapshotResult, Snapshot};
use aws_config::meta::region::RegionProviderChain;
use aws_credential_types::Credentials;
use aws_sdk_s3;
use aws_smithy_types_convert::date_time::DateTimeExt;
use aws_types::region::Region;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{self};
use std::path::PathBuf;

pub struct SnapshotStorage {
    bucket: String,
    client: aws_sdk_s3::Client,
    path_prefix: String,
}

impl SnapshotStorage {
    pub async fn new(config: &AybConfigSnapshots) -> Result<SnapshotStorage, AybError> {
        let mut connection_config = aws_config::from_env().credentials_provider(
            Credentials::from_keys(&config.access_key_id, &config.secret_access_key, None),
        );
        if !config.endpoint_url.is_none() {
            connection_config =
                connection_config.endpoint_url(config.endpoint_url.as_ref().unwrap())
        }

        if !config.region.is_none() {
            let region = Region::new(config.region.clone().unwrap());
            let region_provider = RegionProviderChain::first_try(region).or_default_provider();
            connection_config = connection_config.region(region_provider);
        }

        Ok(SnapshotStorage {
            bucket: config.bucket.clone(),
            client: aws_sdk_s3::Client::new(&connection_config.load().await),
            path_prefix: config.path_prefix.to_string(),
        })
    }

    fn db_path(&self, entity_slug: &str, database_slug: &str, final_part: &str) -> String {
        format!(
            "{}/{}/{}/{}",
            self.path_prefix, entity_slug, database_slug, final_part
        )
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
                            name: object
                                .key
                                .as_ref()
                                .ok_or_else(|| AybError::S3ExecutionError {
                                    message: format!(
                                        "Unable to read key from object: {:?}",
                                        object
                                    ),
                                })?
                                .clone(),
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
        let path = self.db_path(entity_slug, database_slug, &snapshot.snapshot_hash);
        let mut input_file = File::open(snapshot_path)?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        io::copy(&mut input_file, &mut encoder)?;
        let body = aws_sdk_s3::primitives::ByteStream::from(encoder.finish()?);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(path.clone())
            .body(body)
            .send()
            .await
            .or_else(|err| {
                Err(AybError::S3ExecutionError {
                    message: format!("Unable to put snapshot in S3 at {}: {:?}", path, err),
                })
            })?;
        Ok(())
    }
}
