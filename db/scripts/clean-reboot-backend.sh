#!/usr/bin/env bash

set -euo pipefail

# Load environment variables with proper error handling
if [ -f .env ]; then
    source .env
else
    echo "Warning: .env file not found. Using defaults."
    DB_USER=${DB_USER:-ivy}
    DB_PASS=${DB_PASS:-secret_ivy}
    DB_NAME=${DB_NAME:-ivynet}
    DB_PORT=${DB_PORT:-5432}
fi

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
    while [ $retries -gt 0 ]; do
        if command -v pg_isready >/dev/null; then
            if pg_isready -h localhost -p "${DB_PORT}" -U "${DB_USER}"; then
                return 0
            fi
        else
            if docker compose -f backend-compose.yaml exec postgres pg_isready -U "${DB_USER}"; then
                return 0
            fi
        fi
        log "Waiting for PostgreSQL... $((retries-=1)) attempts remaining"
        sleep 1
    done

    log "Error: PostgreSQL failed to start"
    return 1
}

# Main deployment steps
main() {

    local do_versions=false

    for arg in "$@"; do
        case $arg in
            --versions)
                    do_versions=true
                    ;;
            esac
        done

    # Check for docker compose
    if ! command -v docker compose >/dev/null; then
        log "Error: docker compose not found. Please install Docker Desktop for Mac"
        exit 1
    fi

    log "Stopping existing services..."
    docker compose -f backend-compose.yaml down -v || true

    log "Starting services..."
    docker compose -f backend-compose.yaml up -d

    log "Checking PostgreSQL readiness..."
    wait_for_postgres || exit 1

    # Going to proper directory
    cd ..

    log "Running database migrations..."
    if ! sqlx migrate run; then
        log "Error: Migration failed"
        exit 1
    fi

    if [ "${1:-}" = "--prepare" ]; then
        log "Running sqlx prepare..."
        if ! cargo sqlx prepare; then
            log "Error: sqlx prepare failed"
            exit 1
        fi
    fi

    # Going back to backend directory
    cd backend

    log "Adding organization..."
    if ! cargo run -- --add-organization testuser@ivynet.dev:test1234/testorg; then
        log "Error: Failed to add organization"
        exit 1
    fi

    # Fetch node version hashes for valid docker images
    if $do_versions; then
        # Fetch node version hashes for valid docker images
        log "Fetching node version hashes..."
        cargo run -- --add-node-version-hashes

        # Update latest node data versions
        log "Updating node data versions..."
        cargo run -- --update-node-data-versions
    fi

    log "Setting EigenDA breaking change version..."
    cargo run -- --set-breaking-change-version eigenda:holesky:0.8.0:1728622800000

    log "Setup completed successfully"
}

main "$@"
