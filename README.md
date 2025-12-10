# Rust Newsletter API

## Description

This project builds off of [Zero To Production In Rust](https://zero2prod.com) in an attempt to build a rust based API application that handles the newsletter business logic and communicates with a client via JSON-based HTTP requests.

## Pre-requisites

You'll need to install:

- [Rust](https://www.rust-lang.org/tools/install)
- [Docker](https://docs.docker.com/get-docker/)

There are also some OS-specific requirements.

### Windows
  
```bash
cargo install -f cargo-binutils
rustup component add llvm-tools-preview
```

```
cargo install --version="~0.7" sqlx-cli --no-default-features --features rustls,postgres
```

### Linux

```bash
# Ubuntu 
sudo apt-get install lld clang libssl-dev postgresql-client
# Arch 
sudo pacman -S lld clang postgresql
```

```bash
cargo install --version="~0.7" sqlx-cli --no-default-features --features rustls,postgres
```

### MacOS

```bash
brew install michaeleisel/zld/zld
```

```bash
cargo install --version="~0.7" sqlx-cli --no-default-features --features rustls,postgres
```

## How to build

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
cargo build
```

You can now try with opening a browser on http://127.0.0.1:8000/login after
having launch the web server with `cargo run`.

There is a default `admin` account with password
`everythinghastostartsomewhere`. The available entrypoints are listed in
[src/startup.rs](https://github.com/LukeMathWalker/zero-to-production/blob/6bd30650cb8670a146819a342ccefd3d73ed5085/src/startup.rs#L92)

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
