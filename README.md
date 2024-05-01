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



## Use

Until operator registration is ready, please register as an operator using the EigenLayer CLI tool. This tool will check your operator status in order to add you as an operator to individual AVS's, and will check automatically that you are using the correct configuration (eg: CPU cores, memory, storage space) for the requested AVS. 

TODO: Ability to install

For now, 

```sh
cargo build -r
ivy-cli --help 
```



To setup properly first create/import your Ethereum Key

```sh
ivy-cli config create-key [KEYNAME] [PASSWORD] --store
or 
ivy-cli config import-key [PRIVATE-KEY] [KEYNAME] [PASSWORD]
```

Then set your RPC urls for mainnet and holesky

```sh
ivy-cli config set-rpc mainnet [URL]
and
ivy-cli config set-rpc holesky https://rpc.holesky.ethpandaops.io
```

Then try grabbing your stake:

```sh
ivy-cli --network holesky operator get-stake [ADDRESS]
```

and finally booting up the EigenDA AVS!

```sh
ivy-cli --network holesky avs boot eigenda
```

Note: This command assumes you have docker installed, your operator is registered already, and your key has been imported. Also, it downloads files directly from github (eigenda operator setup repository) and two files from AWS that are needed for EigenDA to work (g1.point and g2.point.powerOf2) as well as directly checks your public IP using [api.ipify.org](https://api.ipify.org)

More AVS integrations coming soon!