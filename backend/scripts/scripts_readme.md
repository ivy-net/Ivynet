# Get-Version-Hashes.sh

## Overview

Script fetches Docker image hashes for EigenDA node images across different architectures and generates SQL statements for version tracking.

## Prerequisites

- `skopeo`
- `jq`
- Write permissions for log directory (`/var/log`)

## Usage

```bash
chmod +x version_hash.sh
./version_hash.sh > output.sql
```

## Configuration

Edit these variables at script start:

```bash
IMAGE_URL="docker://ghcr.io/layr-labs/eigenda/opr-node"
NAME="eigenda"
LOG_FILE="/var/log/eigenda_version.log"
```

## Output

- SQL statements in PostgreSQL format
- Logs in `/var/log/eigenda_version.log`

## Database Schema

```sql
CREATE TABLE avs_version_hash (
    avs_type TEXT,
    architecture TEXT,
    hash TEXT,
    version TEXT,
    PRIMARY KEY (avs_type, architecture, version)
);
```

## Error Handling

- Fails on first error (set -e)
- Logs errors with timestamps
- Cleans up temporary files on exit

## Examples

```bash
# Save to file
./version_hash.sh > versions.sql

# Direct database import
./version_hash.sh | psql -d database_name
```


# Clean-Reboot-Backend.sh

## Overview
Script automates the deployment process for the backend service, handling database setup, migrations, and configuration.

## Prerequisites
- Docker and Docker Compose
- PostgreSQL client tools (for health checks)
- Rust toolchain
- sqlx-cli
- Proper permissions to execute scripts

## Configuration
1. Create `.env` file in the same directory:
```env
DB_USER=ivy
DB_PASS=secret_ivy
DB_NAME=ivynet
DB_PORT=5432
```

## Usage

### Basic Deployment
```bash
./deploy.sh
```

### With SQLx Preparation
```bash
./deploy.sh --prepare
```

### Script Behavior
1. Stops existing services
2. Starts fresh containers
3. Waits for PostgreSQL readiness
4. Runs migrations
5. Creates test organization
6. Configures versions
7. Generates version hashes

### Error Handling
- Script stops on first error
- Performs cleanup on interruption
- Logs all operations
- Checks service health

## Troubleshooting
1. If PostgreSQL fails to start, check ports and credentials
2. For permission issues: `chmod +x deploy.sh`
3. Database connection issues: verify `.env` configuration
4. Container conflicts: ensure ports are free

## Maintenance
- Update version numbers in script as needed
- Modify organization credentials for production
- Adjust timeout values if needed

Script docker image sources:

- [X] (Eigenda)[https://github.com/Layr-Labs/eigenda] - `docker://ghcr.io/layr-labs/eigenda/opr-node` - semver
- [X] (Hyperlane)[https://github.com/hyperlane-xyz/hyperlane-monorepo] - `docker://gcr.io/abacus-labs-dev/hyperlane-agent` - not semver
- RED (Brevis)[https://github.com/brevis-network/brevis-avs] - No remote image available
- [X] (Ava)[https://github.com/AvaProtocol/EigenLayer-AVS] - `docker://avaprotocol/ap-avs` - semver + main + latest
- [X] (Lagrange ZK Prover (lgn-coprocessor))[https://github.com/Lagrange-Labs/lgn-coprocessor] - `docker://lagrangelabs/worker` - semver
- [X] (Lagrange State Committees (lsc-node))[https://github.com/Lagrange-Labs/lsc-node] - semver
- ORANGE (Witnesschain Watchtower)[https://github.com/orgs/witnesschain-com/repositories] - ??
- [X] (Predicate)[https://github.com/orgs/PredicateLabs/repositories] - No image available (Binary only)
- [X](Eoracle)[https://github.com/orgs/Eoracle/repositories] - `eoracle-data-validator`
- [X] (K3 Labs)[https://github.com/orgs/k3-labs/repositories] - `k3official/k3-labs-avs-operator` - latest
