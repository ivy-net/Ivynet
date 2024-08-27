WITNESS CONTRACTS NOTES

CONSTANTS
```go
var BlueOrangutan = ChainConfig{
	OperatorRegistryAddress:      common.HexToAddress("0x26710e60A36Ace8A44e1C3D7B33dc8B80eAb6cb7"),
	DiligenceProofManagerAddress: common.HexToAddress("0x7AB3b14F3177935d4539d80289906633615393F2"),
	ChainID:                      *big.NewInt(1237146866),
	BlockExplorer:                "https://blue-orangutan-blockscout.eu-north-2.gateway.fm",
}

var Holesky = ChainConfig{
	OperatorRegistryAddress: common.HexToAddress("0x708CBDDdab358c1fa8efB82c75bB4a116F316Def"),
	WitnessHubAddress:       common.HexToAddress("0xa987EC494b13b21A8a124F8Ac03c9F530648C87D"),
	AVSDirectoryAddress:     common.HexToAddress("0x055733000064333CaDDbC92763c58BF0192fFeBf"),
	ChainID:                 *big.NewInt(17000),
	BlockExplorer:           "https://holesky.etherscan.io",
}

var WitnesschainMainnet = ChainConfig{
	OperatorRegistryAddress:      common.HexToAddress("0xd11e55b821aC8509D2C17f5f76193351252d69aE"),
	DiligenceProofManagerAddress: common.HexToAddress("0x7AB3b14F3177935d4539d80289906633615393F2"),
	BlockExplorer:                "https://explorer.witnesschain.com",
	ChainID:                      *big.NewInt(1702448187),
}

var EthMainnet = ChainConfig{
	OperatorRegistryAddress: common.HexToAddress("0xef1a89841fd189ba28e780a977ca70eb1a5e985d"),
	WitnessHubAddress:       common.HexToAddress("0xD25c2c5802198CB8541987b73A8db4c9BCaE5cC7"),
	AVSDirectoryAddress:     common.HexToAddress("0x135dda560e946695d6f155dacafc6f1f25c1f5af"),
	ChainID:                 *big.NewInt(1),
	BlockExplorer:           "https://etherscan.io",
}
```

### Operator Registry
https://github.com/witnesschain-com/diligencewatchtower-contracts/blob/main/src/core/OperatorRegistry.sol

Implements:
- IOperatorRegistry
- IDelegationManager
- ISlasher
