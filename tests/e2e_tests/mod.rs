mod create_and_query_db_tests;
mod entity_details_and_profile_tests;
mod registration_tests;

pub use create_and_query_db_tests::test_create_and_query_db;
pub use entity_details_and_profile_tests::test_entity_details_and_profile;
pub use registration_tests::test_registration;

const FIRST_ENTITY_SLUG: &str = "e2e-first";
const FIRST_ENTITY_DB: &str = "e2e-first/test.sqlite";
const SECOND_ENTITY_SLUG: &str = "e2e-second";
