#!/bin/bash

export PGPASSWORD=test
export PGHOST=localhost
export PGUSER=postgres_user
rm -rf /tmp/stacks/e2e
dropdb test_db
createdb test_db
sqlx db create
sqlx migrate run
