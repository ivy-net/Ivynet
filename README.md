# The Ivynet Client

<https://ivynet.dev/>

Ivynet is building the operating system for all restaking protocols. Where restaking protocols facilitate a more efficient distribution of restaked assets, Ivynet facilitates an efficient use of compute in order to maxmize yield from those restaked assets.

With the Ivynet client, that begins with calculations determining whether a specific AVS is worth the compute it demands, and then it helps in deploying that AVS.

## Features

- Import, create, and password protect your keys
- Grab information from mainnet and holesky testnet on operators and stakers
- Grab information on your computer/server in relation to AVS's node requirements
- Setup and deploy multiple AVS's in minutes
<!-- - Register as an operator on EigenLayer (Soon) -->


## Ivynet Monorepo

Currently, this repo contains the backend, core, and cli modules of the Ivy platform. The interface is separate as it is not built using Rust. See the links to their individual readme files below. Also, view our docs page at [docs.ivynet.dev](https://docs.ivynet.dev/)

## Build Dependencies

- Rust
- protobuf-compiler (apt install protobuf-compiler)
- Docker (^27.3.1)
- Docker compose (^2.29.7)

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

### Backend Readme:

[here](./backend/README.md)


## AVS Progress

| AVS          | Whitelist         | Deployment Progress        | Blockers                         | Metrics                        |
|--------------|-------------------|----------------------------|----------------------------------|--------------------------------|
| EigenDA      | NA                | Implemented                | NA                               | PR Open                        |
| WitnessChain | Yes, both         | In Progress                | Keyring upgrades                 |                                |
| Omni         | Waiting on Omni   | NA                         | "Final Testnet" releasing soon   |                                |
### Host network mode port detection

Ivynet will automatically detect the the ports used by a given Node running in host network mode by attaching a docker sidecar with netstat installed. This container will run netstat and output ONLY lines which contain valid processes within the scope of the host container. 
