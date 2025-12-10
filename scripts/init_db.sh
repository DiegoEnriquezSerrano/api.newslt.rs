#!/usr/bin/env bash

set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "    cargo install --version='~0.8' sqlx-cli --no-default-features --features rustls,postgres"
  echo >&2 "to install it."
  exit 1
fi

# Check if a custom parameter has been set, otherwise use default values
APP_DB_NAME="${APP_DB_NAME:=newsletter}"
APP_USER_PWD="${APP_USER_PWD:=password}"
APP_USER="${APP_USER:=postgres}"
DB_HOST="${DB_HOST:=localhost}"
DB_PORT="${DB_PORT:=5432}"

export PGPASSWORD="${APP_USER_PWD}"

until psql -h "${DB_HOST}" -U "${APP_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
  echo >&2 "Postgres is still unavailable - sleeping"
  sleep 1
done

echo >&2 "Postgres is up and running on port ${DB_PORT} - running migrations now!"

# Create the application database
DATABASE_URL=postgres://${APP_USER}:${APP_USER_PWD}@${DB_HOST}:${DB_PORT}/${APP_DB_NAME}

export DATABASE_URL

sqlx database create
sqlx migrate run

echo >&2 "Postgres has been migrated, ready to go!"
