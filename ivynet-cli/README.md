Ivynet CLI Documentation

The following is documentation of the various commands that can be called from the Ivynet CLI:

# Init

Initialize the ivyconfig.toml file and perform first-time setup of the ivynet-cli.

Usage:
`ivynet-cli init`

# Config

Manage the ivyconfig.toml file, which is used for base configuraction for the CLI and downstream AVS instances. This allows for piecemeal modification of the setup step done on init.

Usage:
`ivynet-cli config <OP>`

# Operator

Manage the eigenlayer operator. This namespace includes both query actions for operator status of various accounts, as well as management of the operator status of the account's Ethereum address. For write actions, including register, this namespace will use the ECDSA keypair stored in the ivyconfig.toml file to sign transactions.

Usage:
`ivynet-cli operator <OP> <CHAIN> <OTHER_FIELDS>`

# Avs

Setup, run, and manage AVS instances.

Usage:
`ivynet-cli avs <OP> <AVS> <CHAIN>`

Supported operations:
- setup: Run the setup script for the specified AVS. This includes downloading files necessary for the AVS to run, as well as setting up the AVS environment variables.
- optin: Optin to the specified AVS. This will use the stored keypair from the ivyconfig.toml file to optin to the AVS.
- optout: Optout of the specified AVS. This will use the stored keypair from the ivyconfig.toml file to optout of the AVS. UNIMPLEMENTED
- start: Start the specified AVS. This will run the AVS in the background in a docker container. UNIMPLEMENTED
- stop: Stop the specified AVS. This will stop the AVS and close its docker container. UNIMPLEMENTED

Supported AVSes:
- eigenda
- altlayer

Supported chains:
- mainnet
- holesky

Registration steps:
`ivynet-cli operator register holesky` -- Register the operator for Eigenlayer on the holesky chain
`ivynet-cli avs optin eigenda holesky` -- Optin to the Eigenda AVS on the holesky chain

## Quickstart

### Setup and Confgiuration:

The following assumes that the ivynet-cli has been installed and is available in the PATH, and that the user's ECDSA account has already registered as an operator on the Eigenlayer network.

#### Initialize Ivynet:

`ivynet-cli init`

Initialize the Ivnet directory and configuration file. The configuration file can be found at `${HOME}/.ivynet/ivyconfig.toml` and can be configured manually or through `ivynet init` interactive mode. Sensible defaults are provided for newly generated ivyconfig.toml files created via 'empty.'

#### Configure a private key:

If not already done through interactive mode in the `init` command, configure the private key for the ECDSA account that will be used to sign transactions.

To import a private key:

`ivynet-cli config import-key <PRIVATE_KEY>, [KEYNAME], [PASSWORD]`

This will import the private key into the ivyconfig.toml file and create public and private keystore files in the `.ivynet` directory. Private and public keystore files are named `${KEYNAME}.json` and `${KEYNAME}.txt` respectively, and the private keystore file is encrypted with the provided password. Additionally, a `${KEYNAME}.legacy.json` file is created for backwards compatibility with AVS types which expect legacy keystore formats.

Example:
`ivynet-cli config import-key 0x00..01 mykey mypassword`

Alternatively, create a new keypair:

`ivynet-cli config create-key <STORE> [KEYNAME], [PASSWORD]`

Where `[KEYNAME]` and `[PASSWORD]` behave as above, and `[STORE]` is a boolean flag which store the keypair with the above format if true, or simply return the private and public keypair to the console if false.

Example:
`ivynet-cli config create-key true mykey mypassword`

#### Configure RPC endpoints:

If not already done through interactive mode in the `init` command, configure the RPC endpoints for supported networks (currently Mainnet and Holesky.) This can be done by editing the `mainnet_rpc_url` and `holesky_rpc_url` fields in the ivyconfig.toml file, or by running the following commands:

`ivynet-cli config set-rpc <CHAIN> <RPC_URL>`

Example:
`ivynet-cli config set-rpc mainnet https://rpc.flashbots.net`

valid CHAIN values are `mainnet` and `holesky`.

#### Setup the AVS type you wish to run:

`ivynet-cli avs setup <AVS> <CHAIN>`

This will download the necessary files and set up the environment variables for the AVS, as well as create all necessary directories and files for the AVS to run. Setup, configuration files, and executables are stored in the `.eigenlayer/${AVS_NAME}` directory, though additional files may be created elsewhere as a component of the individual AVS setup process, and may vary between AVS types.

Example:
`ivynet-cli avs setup eigenda holesky`

#### Start the Ivynet Daemon:

`ivynet-cli serve [--port <PORT>]`

This will start the ivynet daemon on the specified port, or on port 55501 if no port is specified. The daemon will run in the background.

Example:

`ivynet-cli serve --port 55501`

### Interacting with the Ivy Daemon:


The Ivynet service exposes a GRPC interface for interacting with the daemon, which can be used either via the Ivynet CLI or through GRPC actions directly. Examples are presented using [GRPCurl](https://github.com/fullstorydev/grpcurl)

#### The Avs Namespace
The following GRPC actions are supported:

#### Info

`ivy_daemon_avs.Avs/AvsInfo`
Get information about the currently running AVS instance.

Example call:
`grpcurl -plaintext localhost:55501 ivy_daemon_avs.Avs/AvsInfo`

Returns the follwoing fields:
- active: Whether the AVS is currently active
- avsType: The type of AVS that is currently active
- chain: The chain that the AVS is currently active on

Example return:
```json
{
  "running": true,
  "avsType": "eigenda",
  "chain": "holeksy"
}
```

CLI:
`ivynet-cli avs info`

#### Start

`ivy_daemon_avs.Avs/Start`
Start the loaded AVS instance. This will run the AVS in the background in a docker container.

Example call:
`grpcurl -plaintext localhost:55501 ivy_daemon_avs.Avs/Start`

CLI:
`ivynet-cli avs start`

#### Stop

`ivy_daemon_avs.Avs/Stop`
Stop the AVS instance. This will stop the AVS and close its docker container.

Example call:
`grpcurl -plaintext localhost:55501 ivy_daemon_avs.Avs/Stop`

CLI:
`ivynet-cli avs stop`

#### Optin

`ivy_daemon_avs.Avs/OptIn`
Optin to the AVS instance. This will use the stored keypair from the ivyconfig.toml file to optin to the AVS.

Example call:
`grpcurl -plaintext localhost:55501 ivy_daemon_avs.Avs/OptIn`

CLI:
`ivynet-cli avs optin`

#### Optout

`ivy_daemon_avs.Avs/OptOut`
Optout of the AVS instance. This will use the stored keypair from the ivyconfig.toml file to optout of the AVS.

Example call:
`grpcurl -plaintext localhost:55501 ivy_daemon_avs.Avs/OptOut`

CLI:
`ivynet-cli avs optout`

#### SetAvs

`ivy_daemon_avs.Avs/SetAvs`
Replace the active AVS with a new AVS instance. Errors if the AVS is curently running.

Arguments:
"avs": The name of the AVS to load
"chain": The chain to operate the loaded AVS on

Example:
`grpcurl -plaintext -d '{"avs": "eigenda", "chain": "mainnet"}' localhost:55501 ivy_daemon_avs.Avs/SetAvs`

CLI:
`ivynet-cli avs set <AVS> <CHAIN>`

Example:
`ivynet-cli avs set eigenda holesky`


#### The Operator Namespace

#### Getters

`ivy_daemon_operator.Operator/GetOperator`
