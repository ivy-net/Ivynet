# Backend Setup and Testing Guide

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [sqlx-cli](https://crates.io/crates/sqlx-cli)
- [skopeo](https://github.com/containers/skopeo/blob/main/install.md)
- [jq](https://jqlang.github.io/jq/download/)

## Quick Start (Automated Setup)

```sh
#!/bin/bash
docker compose -f backend-compose.yaml up -d

echo "Waiting for PostgreSQL..."
sleep 5

# Configure environment
export DATABASE_URL=postgresql://ivy:secret_ivy@localhost:5432/ivynet

# Run database migrations and prepare sqlx
sqlx migrate run
cargo sqlx prepare

# TODO: Run the below inside of the testing-compose docker container.
# Initialize test organization and AVS versions
cargo run -- --add-organization testuser@ivynet.dev:test1234/testorg
cargo run -- --set-avs-version eigenda:holesky:0.8.4
cargo run -- --set-breaking-change-version eigenda:holesky:0.8.0:1728622800000

# Get version hashes
chmod +x ./scripts/get_version_hashes.sh
./scripts/get_version_hashes.sh

echo "Setup complete!"
```

## Manual Build Process

1. Start the database:
```sh
docker compose -f backend-compose.yaml up -d
```

2. Configure environment:
```sh
export DATABASE_URL=postgresql://ivy:secret_ivy@localhost:5432/ivynet
```

3. Initialize database:
```sh
sqlx migrate run
cargo sqlx prepare
```

4. Build and run:
```sh
cargo build --release
cargo run --release
```

## Testing Setup

### Docker Compose Testing Environment

For testing the CLI and frontend, use the testing compose file:

```sh
docker compose -f testing-compose.yaml up -d
```

Access Swagger UI documentation at `http://localhost:8080/swagger-ui`

### Email Testing Configuration

1. Create a `.env` file with Sendgrid configuration:
```env
SENDGRID_API_KEY=<YOUR_SENDGRID_API_KEY>
SENDGRID_ORG_VER_TMP=<TEMPLATE_ID_TO_ORG_VERIFICATION_EMAIL>
SENDGRID_USER_VER_TMP=<TEMPLATE_ID_TO_USER_VERIFICATION_EMAIL>
```

2. Required Sendgrid API permissions:
   - Mail Send/Mail Send (Full)
   - Template Engine (RO)

3. Start with email configuration:
```sh
docker compose -f testing-compose.yaml --env-file .env up -d
```

## Development Notes

### Updating SQL Changes

If any of the SQL commands change (in files in `src` folder) the sqlx cache has to be updated.

1. Ensure database is running:
```sh
docker compose -f backend-compose.yaml up -d
```

2. Update sqlx cache:
```sh
export DATABASE_URL=postgresql://ivy:secret_ivy@localhost:5432/ivynet
cargo sqlx prepare
```

This cache update is required for tools like `cargo clippy` to work correctly.

### Clean up-down script

Full script for bringing down and up the backend service from scratch:

```sh
#!/bin/bash

# Cleanup
echo "Stopping and removing docker services and volumes..."
docker compose -f backend-compose.yaml down -v

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
sleep 5  # Simple wait, could be replaced with a more robust check

# Start the docker compose services
echo "Starting docker services..."
docker compose -f backend-compose.yaml up -d

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
sleep 5  # Simple wait, could be replaced with a more robust check

# Set database URL
export DATABASE_URL=postgresql://ivy:secret_ivy@localhost:5432/ivynet
echo "Database URL set to: $DATABASE_URL"

# Run migrations
echo "Running database migrations..."
sqlx migrate run

# sqlx prepare
echo "Running sqlx prepare..."
cargo sqlx prepare

# Add organization
echo "Adding organization..."
cargo run -- --add-organization testuser@ivynet.dev:test1234/testorg

# Set AVS version
echo "Setting AVS version..."
cargo run -- --set-avs-version eigenda:holesky:0.8.4

# Set breaking change version
echo "Setting breaking change version..."
cargo run -- --set-breaking-change-version eigenda:holesky:0.8.0:1728622800000

chmod +x ./scripts/get_version_hashes.sh
./scripts/get_version_hashes.sh

echo "Setup complete!"
```

TODO: 
1. Create run opt which will call find_latest_avs_version and set latest_version in DbAvsVerisonData

2. Create http call which will return a list of Avses for a given X and whether or not they are the latest version
