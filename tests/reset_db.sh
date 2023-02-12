#!/bin/bash

export PGPASSWORD=test
export PGHOST=localhost
rm -rf /tmp/stacks/e2e
dropdb test_db
createdb test_db
sqlx db create
sqlx migrate run
