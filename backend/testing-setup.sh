#!/bin/bash

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

# Add organization
echo "Adding organization..."
cargo run -- --add-organization testuser@ivynet.dev:test1234/testorg

# Set AVS version
echo "Setting AVS version..."
cargo run -- --set-avs-version eigenda:holesky:0.8.4

# Set breaking change version
echo "Setting breaking change version..."
cargo run -- --set-breaking-change-version eigenda:holesky:0.8.0:1728622800000

echo "Setup complete!"