#!/bin/bash

export PGHOST=localhost
export PGUSER=postgres_user
export PGPASSWORD=test
rm -rf ./tests/ayb_data_postgres
rm -rf ./tests/smtp_data
dropdb test_db
createdb test_db
