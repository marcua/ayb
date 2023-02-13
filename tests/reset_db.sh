#!/bin/bash

export PGHOST=localhost
export PGUSER=postgres_user
export PGPASSWORD=test
rm -rf /tmp/stacks/e2e
dropdb test_db
createdb test_db
sqlx db create
sqlx migrate run
