use actix_web::{http::StatusCode, test, App};
use assert_json_diff::assert_json_include;
use serde_json::{json, Value};
use stacks::{hosted_db::QueryResult, http::server::config};
use std::fs;

async fn query_and_assert(query: &'static str, result: &Value) {
    let app = test::init_service(App::new().configure(config)).await;
    let req = test::TestRequest::post()
        .uri("/v1/entity/test.sqlite/query")
        .set_payload(query)
        .to_request();
    let resp = test::call_service(&app, req).await;
    // To print body:
    // assert_eq!(std::str::from_utf8(&test::read_body(resp).await).unwrap(), "expected");
    assert_eq!(resp.status(), StatusCode::OK);

    let body: QueryResult = test::read_body_json(resp).await;
    assert_json_include!(actual: body, expected: result);
}

#[actix_web::test]
async fn test_query_ok() {
    fs::create_dir_all("/tmp/entity").expect("Unable to create database path");
    match fs::remove_file("/tmp/entity/test.sqlite/query") {
        Ok(()) => {}
        Err(err) => {
            assert_eq!(format!("{}", err), "No such file or directory (os error 2)")
        }
    }
    let queries = &[
        (
            "CREATE TABLE test_table(fname varchar, lname varchar);",
            json!({"fields":[], "rows":[]}),
        ),
        (
            "INSERT INTO test_table (fname, lname) VALUES (\"the first\", \"the last\");",
            json!({"fields":[], "rows":[]}),
        ),
        (
            "INSERT INTO test_table (fname, lname) VALUES (\"the first2\", \"the last2\");",
            json!({"fields":[], "rows":[]}),
        ),
        (
            "SELECT * FROM test_table;",
            json!({"fields":
                   ["fname", "lname"],
                   "rows":[["the first", "the last"], ["the first2", "the last2"]]
            }),
        ),
    ];
    for (query, result) in queries {
        query_and_assert(query, result).await;
    }
    fs::remove_file("/tmp/entity/test.sqlite").expect("Unable to clean up test.sqlite");
}
