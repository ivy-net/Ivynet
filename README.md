# The Ivynet CLI tool

https://ivynet.dev/

Ivynet is building the operating system for EigenLayer - where EigenLayer, and underneath it, LRTs, facilitate an efficient use of restaked Ethereum, Ivynet facilitates an efficient use of compute in order to maxmize yield from that staked Eth.

With this cli, that begins with calculations determining whether a specific AVS is worth the compute it demands, and then it helps in deploying that AVS.

## Features

- Import, create, and password protect your keys
- Grab information from mainnet and holesky testnet on operators and stakers
- Grab information on your computer/server in relation to AVS's node requirements
- Register as an operator or staker (Soon)
- Deploy any AVS with one command after utilizing the setup function (Soon - EigenDA coming first)


## Build Dependencies
- Rust
- protobuf-compiler (apt install protobuf-compiler)

## Use

Until operator registration is ready, please register as an operator using the EigenLayer CLI tool. This tool will check your operator status in order to add you as an operator to individual AVS's, and will check automatically that you are using the correct configuration (eg: CPU cores, memory, storage space) for the requested AVS.

TODO: Ability to install, way better documentation, cleanup of core code

NOTE: Development is happening at pace and there may be bugs - please feel free to open a github issue if any are encountered!


### Prepare the client
* Run the build
```sh
cargo clean
cargo build -r
```
* Copy binaries to an accessible place (e.g. `~/bin`)
```sh
[[ -d ~/bin ]] || mkdir ~/bin
cp target/release/ivynet-cli ~/bin
```
* Confirm that the build was successful
```sh
ivynet-cli --help
```

### Prepare Eigenlayer client key

* Install the eigenlayer CLI
```sh
# Install the Eigenlayer CLI:
curl -sSfL https://raw.githubusercontent.com/layr-labs/eigenlayer-cli/master/scripts/install.sh | sh -s
```
* Create a BLS key with optional password:
```sh
eigenlayer operator keys create --key-type bls [keyname]
```

### Setup IvyNet client

* Create or import your Ethereum Key (for now the program does not work with the default 'local' network, so an other network has to be specify with `-n option`)
```sh
ivynet-cli -n holesky config create-key [KEYNAME] [PASSWORD] --store
# or
ivynet-cli -n holesky config import-key [PRIVATE-KEY] [KEYNAME] [PASSWORD]
```
* This will store private and public keyfiles to ${HOME}/.ivynet/ as key_name.json and key_name.txt, respectively.
* Then set the RPC urls for mainnet and/or holesky
```sh
ivynet-cli config set-rpc mainnet [URL]
# and/or
ivynet-cli -n holesky config set-rpc holesky https://rpc.holesky.ethpandaops.io
```

Then try grabbing your stake:

```sh
ivynet-cli --network holesky operator get-stake [ADDRESS]
```


Before runing the EigenDA AVS, perform first-time setup to populate the .env files:

```sh
ivynet-cli config --network holesky avs setup eigenda
```

And finally booting up the EigenDA AVS!

```sh
ivynet-cli --network holesky avs start eigenda
```
Note: This command assumes you have docker installed, your operator is registered already, your ECDSA key has been imported, and your BLS key generated (BLS key can be generated with the EigenLayer CLI). Also, it downloads files directly from github (Ivy's fork of EigenDA operator setup repository) and two files from AWS that are needed for EigenDA to work (g1.point and g2.point.powerOf2) as well as directly checks your public IP using [api.ipify.org](https://api.ipify.org)

More AVS integrations coming soon!

For mac users testing:

```sh
docker pull ghcr.io/layr-labs/eigenda/opr-nodeplugin:0.7.0 --platform=linux/amd64
```
