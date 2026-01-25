# Rust Newsletter API

[![Rust](https://github.com/DiegoEnriquezSerrano/api.newslt.rs/actions/workflows/general.yml/badge.svg)](https://github.com/DiegoEnriquezSerrano/api.newslt.rs/actions/workflows/general.yml)

## Description

This project builds off of [Zero To Production In Rust](https://zero2prod.com) in an attempt to build a rust based API application that handles the newsletter business logic and communicates with a client via JSON-based HTTP requests.

## Pre-requisites

You'll need to install:

- [Rust](https://www.rust-lang.org/tools/install)
- [Docker](https://docs.docker.com/get-docker/)
- [Postgresql](https://www.postgresql.org/download/) >= v17

There are also some OS-specific requirements.

### Windows

```bash
cargo install -f cargo-binutils
rustup component add llvm-tools-preview
```

```bash
cargo install --version="~0.8" sqlx-cli --no-default-features --features rustls,postgres
```

### Linux

```bash
# Ubuntu
sudo apt-get install lld clang libssl-dev
# Arch
sudo pacman -S lld clang
```

```bash
cargo install --version="~0.8" sqlx-cli --no-default-features --features rustls,postgres
```

### MacOS

```bash
brew install michaeleisel/zld/zld
```

```bash
cargo install --version="~0.8" sqlx-cli --no-default-features --features rustls,postgres
```

## How to build

Start services (Postgres, Redis, Mailpit, RustFS) via Docker compose:

```bash
docker compose up -d --remove-orphans
```

Launch a (migrated) Postgres database:

```bash
./scripts/init_db.sh
```

Start web server:

```bash
cargo run
```

or, alternatively, use convenience script to watch for changes and automatically run formatter, linter, and test suite

```bash
./scripts/dev_loop.sh
```

Create initial user:

```bash
cargo run --bin superuser
```

This CLI program will prompt for a username, email and confirmed password and create a record in the `users` table.

## How to test

Start Postgres and Redis services via Docker compose:

```bash
docker compose up -d --remove-orphans
```

Launch a (migrated) Postgres database:

```bash
./scripts/init_db.sh
```

Launch `cargo`:

```bash
cargo test
```
