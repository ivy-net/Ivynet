#!/bin/bash
set -euo pipefail

# Load environment variables
source .env 2>/dev/null || {
    echo "Warning: .env file not found. Using defaults."
    export DB_USER=${DB_USER:-ivy}
    export DB_PASS=${DB_PASS:-secret_ivy}
    export DB_NAME=${DB_NAME:-ivynet}
    export DB_PORT=${DB_PORT:-5432}
}

export DATABASE_URL="postgresql://${DB_USER}:${DB_PASS}@localhost:${DB_PORT}/${DB_NAME}"

# Logging
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $1"
}

# Cleanup handler
cleanup() {
    log "Cleaning up..."
    docker compose -f backend-compose.yaml down -v
    exit
}
trap cleanup SIGINT SIGTERM

# Health check function
wait_for_postgres() {
    local retries=30
    until pg_isready -h localhost -p "${DB_PORT}" -U "${DB_USER}" || [ $retries -eq 0 ]; do
        log "Waiting for PostgreSQL... $((retries-=1)) attempts remaining"
        sleep 1
    done

    if [ $retries -eq 0 ]; then
        log "Error: PostgreSQL failed to start"
        exit 1
    fi
}

# Main deployment steps
main() {
    log "Stopping existing services..."
    docker compose -f backend-compose.yaml down -v

    log "Starting services..."
    docker compose -f backend-compose.yaml up -d

    log "Checking PostgreSQL readiness..."
    wait_for_postgres

    log "Running database migrations..."
    sqlx migrate run || {
        log "Error: Migration failed"
        exit 1
    }

    if [ "${1:-}" = "--prepare" ]; then
        log "Running sqlx prepare..."
        cargo sqlx prepare || {
            log "Error: sqlx prepare failed"
            exit 1
        }
    fi

    log "Adding organization..."
    cargo run -- --add-organization testuser@ivynet.dev:test1234/testorg || {
        log "Error: Failed to add organization"
        exit 1
    }

    log "Configuring versions..."
    cargo run -- --set-avs-version eigenda:holesky:0.8.4
    cargo run -- --set-breaking-change-version eigenda:holesky:0.8.0:1728622800000

    log "Running version hash script..."
    chmod +x ./scripts/get_version_hashes_new.sh
    ./scripts/get_version_hashes_new.sh

    log "Setup completed successfully"
}

main "$@"
