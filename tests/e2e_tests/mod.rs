mod create_and_query_db_tests;
mod entity_details_and_profile_tests;
mod health_check_tests;
mod permissions_tests;
mod registration_tests;
mod snapshot_tests;
mod token_tests;

pub use create_and_query_db_tests::test_create_and_query_db;
pub use entity_details_and_profile_tests::test_entity_details_and_profile;
pub use health_check_tests::test_health_check;
pub use permissions_tests::test_permissions;
pub use registration_tests::test_registration;
pub use snapshot_tests::test_snapshots;
pub use token_tests::test_tokens;

const FIRST_ENTITY_DB: &str = "e2e-first/test.sqlite";
const FIRST_ENTITY_DB_CASED: &str = "E2E-FiRST/test.sqlite";
const FIRST_ENTITY_DB2: &str = "e2e-first/another.sqlite";
const FIRST_ENTITY_DB_SLUG: &str = "test.sqlite";
const FIRST_ENTITY_SLUG: &str = "e2e-first";
const FIRST_ENTITY_SLUG_CASED: &str = "E2E-FiRsT";
const SECOND_ENTITY_SLUG: &str = "e2e-second";
const THIRD_ENTITY_SLUG: &str = "e2e-third";
