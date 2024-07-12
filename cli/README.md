Ivynet CLI Documentation

The following is documentation of the various commands that can be called from the Ivynet CLI:

# Init

Initialize the ivyconfig.toml file and perform first-time setup of the ivynet cli.

Usage:
`ivynet init`

# Config

Manage the ivyconfig.toml file, which is used for base configuraction for the CLI and downstream AVS instances. This allows for piecemeal modification of the setup step done on init.

Usage:
`ivynet config <OP>`

# Operator

Manage the eigenlayer operator. This namespace includes both query actions for operator status of various accounts, as well as management of the operator status of the account's Ethereum address. For write actions, including register, this namespace will use the ECDSA keypair stored in the ivyconfig.toml file to sign transactions.

Usage:
`ivynet operator <OP> <CHAIN> <OTHER_FIELDS>`

# Avs

Setup, run, and manage AVS instances.

Usage:
`ivynet avs <OP> <AVS> <CHAIN>`

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
`ivynet operator register holesky` -- Register the operator for Eigenlayer on the holesky chain
`ivynet avs optin eigenda holesky` -- Optin to the Eigenda AVS on the holesky chain

## Quickstart

### Setup and Confgiuration:

The following assumes that the ivynet has been installed and is available in the PATH, and that the user's ECDSA account has already registered as an operator on the Eigenlayer network.

#### Initialize Ivynet:

`ivynet init`

Initialize the Ivnet directory and configuration file. The configuration file can be found at `${HOME}/.ivynet/ivyconfig.toml` and can be configured manually or through `ivynet init` interactive mode. Sensible defaults are provided for newly generated ivyconfig.toml files created via 'empty' mode.

#### Configure a private key:

If not already done through interactive mode in the `init` command, configure the private key for the ECDSA account that will be used to sign transactions.

To import a private key:

`ivynet config import-key <PRIVATE_KEY>, [KEYNAME], [PASSWORD]`

This will import the private key into the ivyconfig.toml file and create public and private keystore files in the `.ivynet` directory. Private and public keystore files are named `${KEYNAME}.json` and `${KEYNAME}.txt` respectively, and the private keystore file is encrypted with the provided password. Additionally, a `${KEYNAME}.legacy.json` file is created for backwards compatibility with AVS types which expect legacy keystore formats.

Example:
`ivynet config import-key 0x00..01 mykey mypassword`

Alternatively, create a new keypair:

`ivynet config create-key <STORE> [KEYNAME], [PASSWORD]`

Where `[KEYNAME]` and `[PASSWORD]` behave as above, and `[STORE]` is a boolean flag which store the keypair with the above format if true, or simply return the private and public keypair to the console if false.

Example:
`ivynet config create-key true mykey mypassword`

#### Configure RPC endpoints:

If not already done through interactive mode in the `init` command, configure the RPC endpoints for supported networks (currently Mainnet and Holesky.) This can be done by editing the `mainnet_rpc_url` and `holesky_rpc_url` fields in the ivyconfig.toml file, or by running the following commands:

`ivynet config set-rpc <CHAIN> <RPC_URL>`

Example:
`ivynet config set-rpc mainnet https://rpc.flashbots.net`

valid CHAIN values are `mainnet` and `holesky`.

#### Setup the AVS type you wish to run:

`ivynet avs setup <AVS> <CHAIN>`

This will download the necessary files and set up the environment variables for the AVS, as well as create all necessary directories and files for the AVS to run. Setup, configuration files, and executables are stored in the `.eigenlayer/${AVS_NAME}` directory, though additional files may be created elsewhere as a component of the individual AVS setup process, and may vary between AVS types.

Example:
`ivynet avs setup eigenda holesky`

#### Start the Ivynet Daemon:

`ivynet serve`

This will start the ivynet daemon over a unix domain socket, located at `~{HOME}/.ivynet/ivynet.ipc`. The daemon will run in the background.

### Interacting with the Ivy Daemon:


The Ivynet service exposes a GRPC interface for interacting with the daemon, which can be used either via the Ivynet CLI or through GRPC actions directly. Examples are presented using [GRPCurl](https://github.com/fullstorydev/grpcurl)

#### Using GRPCurl:

GRPCurl can be used to access the GRPC interface of the Ivynet daemon, either through IPC or an exposed port. The following examples assume that the Ivynet daemon is running on an IPC at `${HOME}/.ivynet/ivynet.ipc` (the default location).

List GRPC services:
```
grpcurl -unix -plaintext -authority "localhost" ~/.ivynet/ivynet.ipc list
```

In some later versions of GRPCurl, the `-authority` flag may be unnecessary, but is included here for robustness.

### The Avs Namespace
The following GRPC actions are supported:

#### Info

`ivy_daemon_avs.Avs/AvsInfo`
Get information about the currently running AVS instance.

Return:
```json
{
  /**
    * Whether the AVS is currently running
    * @type {boolean}
    */
  "running": true,
  /**
    * The type of AVS that is currently running
    * @type {string}
    */
  "avsType": "eigenda",
  /**
    * The chain that the AVS is currently running on
    * @type {string}
    */
  "chain": "holeksy"
}
```

CLI:
`ivynet avs info`

GRPCurl:

`grpcurl -unix -plaintext -authority "localhost" ~/.ivynet/ivynet.ipc avs.Avs/AvsInfo`

#### Select

`ivy_daemon_avs.Avs/Select`
Replace the active AVS with a new AVS instance. Errors if the AVS is curently running.

Arguments:
"avs": The name of the AVS to load
"chain": The chain to operate the loaded AVS on

CLI:

`ivynet avs select <AVS> <CHAIN>`

GRPCurl:

`grpcurl -unix -plaintext -authority "localhost" -d '{"avs": "eigenda", "chain": "mainnet"}'~/.ivynet/ivynet.ipc avs.Avs/SelectAvs`

Example:

`ivynet avs select eigenda holesky`

#### Start

`avs.Avs/Start`
Start the loaded AVS instance. This will run the AVS in the background in a docker container. Errors if no AVS has been selected or the AVS is already running.

CLI:

`ivynet avs start`

GRPCurl:

`grpcurl -unix -plaintext -authority "localhost" ~/.ivynet/ivynet.ipc avs.Avs/Start`

#### Stop

`ivy_daemon_avs.Avs/Stop`
Stop the AVS instance. This will stop the AVS and close its docker container.

CLI:
`ivynet avs stop`

GRPCurl:
`grpcurl -unix -plaintext -authority "localhost" ~/.ivynet/ivynet.ipc avs.Avs/Stop`

#### Optin

`ivy_daemon_avs.Avs/OptIn`
Optin to the AVS instance. This will use the stored keypair from the ivyconfig.toml file to optin to the AVS. Errors if no AVS has been selected or the AVS is already running.

CLI:
`ivynet avs optin`

GRPCurl:

`grpcurl -unix -plaintext -authority "localhost" ~/.ivynet/ivynet.ipc avs.Avs/OptIn`

#### Optout

`ivy_daemon_avs.Avs/OptOut`
Optout of the AVS instance. This will use the stored keypair from the ivyconfig.toml file to optout of the AVS. Errors if no AVS has been selected or the AVS is already running.

CLI:

`ivynet avs optout`

GRPCurl:

```grpcurl -unix -plaintext -authority "localhost" ~/.ivynet/ivynet.ipc avs.Avs/OptOut```


### The Operator Namespace

### Getters

#### Operator Details

`ivy_daemon_operator.Operator/GetOperatorDetails`
Get operator details for the currently loaded operator, defined by the ECDSA keypair file referenced in ivyconfig.toml.

Return:
```json
{
  /**
    * The Ethereum address of the operator
    * @type {string}
    */
  "operator": "0x00000000000000000000000000000000DeaDBeef",
  /**
    * Whether the operator is registered on the Eigenlayer network
    * @type {boolean}
    */
  "is_registered": true,
  /**
    * The earnings receiver for the operator. Currently deprecated by the Eigenlayer network but maintained for backwards compatibility.
    * @type {string}
    */
  "__deprecated_earnings_receiver": "0x0000000000000000000000000000000000000000",
  /**
    * The address of the operator's delegation approver. This is the address that can approve or deny delegation requests.
    * @type {string}
    */
  "delegation_approver": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
  /**
    * The number of blocks that this operator's delegated stakers must wait before opting out of their delegation.
    * @type {number}
    */
  "staker_opt_out_window_blocks": 10,
}
```

CLI:
`ivynet operator get details`

GRPCurl:
`grpcurl -unix -plaintext -authority "localhost" ~/.ivynet/ivynet.ipc operator.Operator/GetOperatorDetails`

#### Operator Shares

`ivy_daemon_operator.Operator/GetOperatorShares`
Get the operator shares for the currently loaded operator.

Return:
```json
{
  /**
    * The operator shares for the operator. This is a list of objects, each containing a strategy and the number of shares the operator has in that strategy. This returns an array of all available strategies, even if the operator has no shares in them.
    * @type {Array.<{strategy: string, shares: string}>}
    */
  "operatorShares": [
    {
      /**
        * The strategy that the operator has shares in
        * @type {string}
        */
      "strategy": "0x7d704507b76571a51d9cae8addabbfd0ba0e63d3",
      /**
        * The number of shares that the operator has in the strategy
        * @type {string}
        */
      "shares": "100000000000000000"
    },
    ...
  ]
}
```

CLI:
`ivynet operator get shares`

GRPCurl:
`grpcurl -unix -plaintext -authority "localhost" ~/.ivynet/ivynet.ipc operator.Operator/GetOperatorShares`

#### Operator Delgatable Shares

`ivy_daemon_operator.Operator/GetDelegatableShares`
Get the operator's delegatable shares. These are the shares that the operator can delegate to other stakers.

Return:
```json
{
  /**
    * The operator's delegatable shares. This is a list of objects, each containing a strategy and the number of shares the operator has in that strategy that are delegatable. This returns an array of only the strategies that the operator has delegatable shares in.
    * @type {Array.<{strategy: string, shares: string}>}
    */
  "delegatableShares": [
    {
      /**
        * The strategy that the operator has delegatable shares in
        * @type {string}
        */
      "strategy": "0x7d704507b76571a51d9cae8addabbfd0ba0e63d3",
      /**
        * The number of shares that the operator has in the strategy that are delegatable
        * @type {string}
        */
      "shares": "100000000000000000"
    },
    ...
  ]
}
```

CLI:
`ivynet operator get delegatable-shares`

GRPCurl:
`grpcurl -unix -plaintext -authority "localhost" ~/.ivynet/ivynet.ipc operator.Operator/GetDelegatableShares`
