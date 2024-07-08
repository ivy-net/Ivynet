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
