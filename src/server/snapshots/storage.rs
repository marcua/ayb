use crate::error::AybError;
use crate::server::config::AybConfigSnapshots;
use crate::server::snapshots::models::{InstantiatedSnapshot, ListSnapshotResult};
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;

pub struct SnapshotStorage {
    bucket: Bucket,
    path_prefix: String,
}

impl SnapshotStorage {
    pub fn new(config: &AybConfigSnapshots) -> Result<SnapshotStorage, AybError> {
        let credentials = Credentials::new(
            Some(&config.access_key_id),
            Some(&config.secret_access_key),
            None,
            None,
            None,
        )?;
        let region_slug = config.region.unwrap_or("us-east-1".to_string());
        let region: Region = match config.endpoint {
            Some(endpoint) => Region::Custom {
                region: region_slug,
                endpoint: endpoint,
            },
            None => region_slug.parse()?,
        };

        Ok(SnapshotStorage {
            bucket: Bucket::new(&config.bucket, region, credentials)?,
            path_prefix: config.path_prefix,
        })
    }
    fn db_path(self, entity_slug: &str, database_slug: &str, final_path: &str) -> String {
        format!(
            "{}/{}/{}/{}",
            self.path_prefix, entity_slug, database_slug, final_path
        )
    }

    // switch to rusty: https://docs.rs/rusty-s3/latest/rusty_s3/
    // or https://docs.rs/object_store/latest/object_store/aws/struct.AmazonS3Builder.html
    // Start with object_store
    // Run minio
    // Get put path working first
    // Then get list working
    pub async fn list_snapshots(
        self,
        entity_slug: &str,
        database_slug: &str,
    ) -> Result<Vec<InstantiatedSnapshot>, AybError> {
        Ok(self
            .bucket
            .list(
                self.db_path(entity_slug, database_slug, ""),
                Some("/".to_string()),
            )
            .await?
            .contents
            .iter()
            .map(|&result| ListSnapshotResult {
                last_modified_at: result.last_modified,
                snapshot_hash: result.key,
            }))
    }
}
