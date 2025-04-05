# The Ivynet Client

<https://ivynet.dev/>

Ivynet is building a unified observability solution for all crypto node operators. Primarily, the Ivynet cli daemon is used to grab all information from any actively deployed AVSs it detects (or manually added ones) and push data upwards to the ingress service. This data is then transformed and enriched with eg: active set data and accessed by the api service.

## Ivynet Monorepo

Currently, this repo contains the api, ingress, scanner, and cli modules of the Ivy platform, as well as their dependencies. The interface is separate as it is not built using Rust. See the links to their individual readme files below. Also, view our docs page at [docs.ivynet.dev](https://docs.ivynet.dev/)

## Build Dependencies

- Rust
- protobuf-compiler (apt install protobuf-compiler)
- Docker (^27.3.1)
- Docker compose (^2.29.7)

## DevOps documentation

To learn more about GitHub Actions, PreCommit settings, extra scripts and tools check the [DevOps documentation](devops/README.md).

## Use

Until operator registration is ready, please register as an operator using the EigenLayer CLI tool. This tool will check your operator status in order to add you as an operator to individual AVS's, and will check automatically that you are using the correct configuration (eg: CPU cores, memory, storage space) for the requested AVS.

NOTE: Development is happening at pace and there may be bugs - please feel free to open a github issue if any are encountered!

### Prepare the client

- Run the build

```sh
cargo clean
cargo build -r
```

- Copy binaries to an accessible place (e.g. `~/bin`)

```sh
[[ -d ~/bin ]] || mkdir ~/bin
cp target/release/ivynet ~/bin
```

- Confirm that the build was successful

```sh
ivynet --help
```

### Prepare Eigenlayer BLS key

- Install the eigenlayer CLI

```sh
# Install the Eigenlayer CLI:
curl -sSfL https://raw.githubusercontent.com/layr-labs/eigenlayer-cli/master/scripts/install.sh | sh -s
```

- Create a BLS key with optional password:

```sh
eigenlayer operator keys create --key-type bls [keyname]
```

### Setup IvyNet client

Please refer to the CLI documentation [here](./cli/README.md)



### Backend Setup and Testing Guide

1. Bring up the postgres database container:

Optional: Ensure that no existing instance of the database is running:

```sh
docker compose -f ./ivynet-database/backend-compose.yaml down -v
```

Bring up the database:

```sh
docker compose -f ./ivynet-database/backend-compose.yaml up -d
```

2. Set the DATABASE_URL environment variable:

```sh
export DATABASE_URL=postgresql://ivy:secret_ivy@localhost:5432/ivynet
```

3. Run database migrations and prepare sqlx:

```sh
sqlx migrate run
cargo sqlx prepare --workspace
```

4. Initialize test organization and AVS versions from backend, and register your node:

```sh
cd api
cargo run -- --add-organization testuser@ivynet.dev:test1234/testorg
cd ..
cargo run register-node
```

5. Populate version hashes table from remote docker repositories and latest node versions table:

```sh
cargo run --bin ivynet-backend -- --add-node-version-hashes
cargo run --bin ivynet-backend -- --update-node-data-versions
```

6. Run ingress from ./ingress:
```sh
cd ingress
cargo run
```

7. Run the api from ./api:
```sh
cd backend
cargo run
```

8. Run the onchain data scanner from ./scanner:

```sh
cd scanner
cargo run
```

Convenient shell script for confiuration-related steps

```sh
echo "Closing existing database..."
docker compose -f ./ivynet-database/backend-compose.yaml down -v
sleep 3
docker compose -f ./ivynet-database/backend-compose.yaml up -d
echo "Waiting for PostgreSQL database..."
sleep 3
export DATABASE_URL=postgresql://ivy:secret_ivy@localhost:5432/ivynet
sqlx migrate run
cargo sqlx prepare --workspace
```

Once the backend is running and your monitor process is configured, you can register your machine with the backend:

```sh
cargo run register-node
```

### Host network mode port detection

Ivynet will automatically detect the the ports used by a given Node running in host network mode by attaching a docker sidecar with netstat installed. This container will run netstat and output ONLY lines which contain valid processes within the scope of the host container.
