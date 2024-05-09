use ethers_contract::abigen;
use ethers_core::abi::Address;

use crate::rpc_management::{self, Network};

pub type StakeRegistry = StakeRegistryAbi<rpc_management::Client>;
pub type RegistryCoordinator = RegistryCoordinatorAbi<rpc_management::Client>;
pub type RegistryCoordinatorSigner = RegistryCoordinatorAbi<rpc_management::Signer>;

pub fn setup_stake_registry() -> StakeRegistry {
    let stake_reg_addr: Address = get_stake_registry_address().parse().expect("Could not parse StakeRegistry address");
    StakeRegistryAbi::new(stake_reg_addr.clone(), rpc_management::get_client())
}

pub fn setup_registry_coordinator() -> RegistryCoordinator {
    let stake_reg_addr: Address =
        get_registry_coordinator_address().parse().expect("Could not parse RegistryCoordinator address");
    RegistryCoordinatorAbi::new(stake_reg_addr.clone(), rpc_management::get_client())
}

pub fn setup_registry_coordinator_signer() -> RegistryCoordinatorSigner {
    let stake_reg_addr: Address =
        get_registry_coordinator_address().parse().expect("Could not parse RegistryCoordinator address");
    RegistryCoordinatorAbi::new(stake_reg_addr.clone(), rpc_management::get_signer())
}

pub fn get_stake_registry_address() -> String {
    match rpc_management::get_network() {
        Network::Mainnet => "0x006124ae7976137266feebfb3f4d2be4c073139d".to_string(),
        Network::Holesky => "0xBDACD5998989Eec814ac7A0f0f6596088AA2a270".to_string(),
        Network::Local => todo!(),
    }
}

pub fn get_registry_coordinator_address() -> String {
    match rpc_management::get_network() {
        Network::Mainnet => "0x0baac79acd45a023e19345c352d8a7a83c4e5656".to_string(),
        Network::Holesky => "0x53012C69A189cfA2D9d29eb6F19B32e0A2EA3490".to_string(),
        Network::Local => todo!(),
    }
}

abigen!(
    RegistryCoordinatorAbi,
    r#"[
        {
            "type": "constructor",
            "inputs": [
                {
                    "name": "_serviceManager",
                    "type": "address",
                    "internalType": "contract IServiceManager"
                },
                {
                    "name": "_stakeRegistry",
                    "type": "address",
                    "internalType": "contract IStakeRegistry"
                },
                {
                    "name": "_blsApkRegistry",
                    "type": "address",
                    "internalType": "contract IBLSApkRegistry"
                },
                {
                    "name": "_indexRegistry",
                    "type": "address",
                    "internalType": "contract IIndexRegistry"
                }
            ],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "OPERATOR_CHURN_APPROVAL_TYPEHASH",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "bytes32",
                    "internalType": "bytes32"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "PUBKEY_REGISTRATION_TYPEHASH",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "bytes32",
                    "internalType": "bytes32"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "blsApkRegistry",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "contract IBLSApkRegistry"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "calculateOperatorChurnApprovalDigestHash",
            "inputs": [
                {
                    "name": "registeringOperator",
                    "type": "address",
                    "internalType": "address"
                },
                {
                    "name": "registeringOperatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "operatorKickParams",
                    "type": "tuple[]",
                    "internalType": "struct IRegistryCoordinator.OperatorKickParam[]",
                    "components": [
                        {
                            "name": "quorumNumber",
                            "type": "uint8",
                            "internalType": "uint8"
                        },
                        {
                            "name": "operator",
                            "type": "address",
                            "internalType": "address"
                        }
                    ]
                },
                {
                    "name": "salt",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "expiry",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "bytes32",
                    "internalType": "bytes32"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "churnApprover",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "createQuorum",
            "inputs": [
                {
                    "name": "operatorSetParams",
                    "type": "tuple",
                    "internalType": "struct IRegistryCoordinator.OperatorSetParam",
                    "components": [
                        {
                            "name": "maxOperatorCount",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "kickBIPsOfOperatorStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        },
                        {
                            "name": "kickBIPsOfTotalStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        }
                    ]
                },
                {
                    "name": "minimumStake",
                    "type": "uint96",
                    "internalType": "uint96"
                },
                {
                    "name": "strategyParams",
                    "type": "tuple[]",
                    "internalType": "struct IStakeRegistry.StrategyParams[]",
                    "components": [
                        {
                            "name": "strategy",
                            "type": "address",
                            "internalType": "contract IStrategy"
                        },
                        {
                            "name": "multiplier",
                            "type": "uint96",
                            "internalType": "uint96"
                        }
                    ]
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "deregisterOperator",
            "inputs": [
                {
                    "name": "quorumNumbers",
                    "type": "bytes",
                    "internalType": "bytes"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "ejectOperator",
            "inputs": [
                {
                    "name": "operator",
                    "type": "address",
                    "internalType": "address"
                },
                {
                    "name": "quorumNumbers",
                    "type": "bytes",
                    "internalType": "bytes"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "ejector",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getCurrentQuorumBitmap",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint192",
                    "internalType": "uint192"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getOperator",
            "inputs": [
                {
                    "name": "operator",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "tuple",
                    "internalType": "struct IRegistryCoordinator.OperatorInfo",
                    "components": [
                        {
                            "name": "operatorId",
                            "type": "bytes32",
                            "internalType": "bytes32"
                        },
                        {
                            "name": "status",
                            "type": "uint8",
                            "internalType": "enum IRegistryCoordinator.OperatorStatus"
                        }
                    ]
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getOperatorFromId",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getOperatorId",
            "inputs": [
                {
                    "name": "operator",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "bytes32",
                    "internalType": "bytes32"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getOperatorSetParams",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "tuple",
                    "internalType": "struct IRegistryCoordinator.OperatorSetParam",
                    "components": [
                        {
                            "name": "maxOperatorCount",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "kickBIPsOfOperatorStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        },
                        {
                            "name": "kickBIPsOfTotalStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        }
                    ]
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getOperatorStatus",
            "inputs": [
                {
                    "name": "operator",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint8",
                    "internalType": "enum IRegistryCoordinator.OperatorStatus"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getQuorumBitmapAtBlockNumberByIndex",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "blockNumber",
                    "type": "uint32",
                    "internalType": "uint32"
                },
                {
                    "name": "index",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint192",
                    "internalType": "uint192"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getQuorumBitmapHistoryLength",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getQuorumBitmapIndicesAtBlockNumber",
            "inputs": [
                {
                    "name": "blockNumber",
                    "type": "uint32",
                    "internalType": "uint32"
                },
                {
                    "name": "operatorIds",
                    "type": "bytes32[]",
                    "internalType": "bytes32[]"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint32[]",
                    "internalType": "uint32[]"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getQuorumBitmapUpdateByIndex",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "index",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "tuple",
                    "internalType": "struct IRegistryCoordinator.QuorumBitmapUpdate",
                    "components": [
                        {
                            "name": "updateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "nextUpdateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "quorumBitmap",
                            "type": "uint192",
                            "internalType": "uint192"
                        }
                    ]
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "indexRegistry",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "contract IIndexRegistry"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "initialize",
            "inputs": [
                {
                    "name": "_initialOwner",
                    "type": "address",
                    "internalType": "address"
                },
                {
                    "name": "_churnApprover",
                    "type": "address",
                    "internalType": "address"
                },
                {
                    "name": "_ejector",
                    "type": "address",
                    "internalType": "address"
                },
                {
                    "name": "_pauserRegistry",
                    "type": "address",
                    "internalType": "contract IPauserRegistry"
                },
                {
                    "name": "_initialPausedStatus",
                    "type": "uint256",
                    "internalType": "uint256"
                },
                {
                    "name": "_operatorSetParams",
                    "type": "tuple[]",
                    "internalType": "struct IRegistryCoordinator.OperatorSetParam[]",
                    "components": [
                        {
                            "name": "maxOperatorCount",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "kickBIPsOfOperatorStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        },
                        {
                            "name": "kickBIPsOfTotalStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        }
                    ]
                },
                {
                    "name": "_minimumStakes",
                    "type": "uint96[]",
                    "internalType": "uint96[]"
                },
                {
                    "name": "_strategyParams",
                    "type": "tuple[][]",
                    "internalType": "struct IStakeRegistry.StrategyParams[][]",
                    "components": [
                        {
                            "name": "strategy",
                            "type": "address",
                            "internalType": "contract IStrategy"
                        },
                        {
                            "name": "multiplier",
                            "type": "uint96",
                            "internalType": "uint96"
                        }
                    ]
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "isChurnApproverSaltUsed",
            "inputs": [
                {
                    "name": "",
                    "type": "bytes32",
                    "internalType": "bytes32"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "bool",
                    "internalType": "bool"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "numRegistries",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "owner",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "pause",
            "inputs": [
                {
                    "name": "newPausedStatus",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "pauseAll",
            "inputs": [],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "paused",
            "inputs": [
                {
                    "name": "index",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "bool",
                    "internalType": "bool"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "paused",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "pauserRegistry",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "contract IPauserRegistry"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "pubkeyRegistrationMessageHash",
            "inputs": [
                {
                    "name": "operator",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "tuple",
                    "internalType": "struct BN254.G1Point",
                    "components": [
                        {
                            "name": "X",
                            "type": "uint256",
                            "internalType": "uint256"
                        },
                        {
                            "name": "Y",
                            "type": "uint256",
                            "internalType": "uint256"
                        }
                    ]
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "quorumCount",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "quorumUpdateBlockNumber",
            "inputs": [
                {
                    "name": "",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "registerOperator",
            "inputs": [
                {
                    "name": "quorumNumbers",
                    "type": "bytes",
                    "internalType": "bytes"
                },
                {
                    "name": "socket",
                    "type": "string",
                    "internalType": "string"
                },
                {
                    "name": "params",
                    "type": "tuple",
                    "internalType": "struct IBLSApkRegistry.PubkeyRegistrationParams",
                    "components": [
                        {
                            "name": "pubkeyRegistrationSignature",
                            "type": "tuple",
                            "internalType": "struct BN254.G1Point",
                            "components": [
                                {
                                    "name": "X",
                                    "type": "uint256",
                                    "internalType": "uint256"
                                },
                                {
                                    "name": "Y",
                                    "type": "uint256",
                                    "internalType": "uint256"
                                }
                            ]
                        },
                        {
                            "name": "pubkeyG1",
                            "type": "tuple",
                            "internalType": "struct BN254.G1Point",
                            "components": [
                                {
                                    "name": "X",
                                    "type": "uint256",
                                    "internalType": "uint256"
                                },
                                {
                                    "name": "Y",
                                    "type": "uint256",
                                    "internalType": "uint256"
                                }
                            ]
                        },
                        {
                            "name": "pubkeyG2",
                            "type": "tuple",
                            "internalType": "struct BN254.G2Point",
                            "components": [
                                {
                                    "name": "X",
                                    "type": "uint256[2]",
                                    "internalType": "uint256[2]"
                                },
                                {
                                    "name": "Y",
                                    "type": "uint256[2]",
                                    "internalType": "uint256[2]"
                                }
                            ]
                        }
                    ]
                },
                {
                    "name": "operatorSignature",
                    "type": "tuple",
                    "internalType": "struct ISignatureUtils.SignatureWithSaltAndExpiry",
                    "components": [
                        {
                            "name": "signature",
                            "type": "bytes",
                            "internalType": "bytes"
                        },
                        {
                            "name": "salt",
                            "type": "bytes32",
                            "internalType": "bytes32"
                        },
                        {
                            "name": "expiry",
                            "type": "uint256",
                            "internalType": "uint256"
                        }
                    ]
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "registerOperatorWithChurn",
            "inputs": [
                {
                    "name": "quorumNumbers",
                    "type": "bytes",
                    "internalType": "bytes"
                },
                {
                    "name": "socket",
                    "type": "string",
                    "internalType": "string"
                },
                {
                    "name": "params",
                    "type": "tuple",
                    "internalType": "struct IBLSApkRegistry.PubkeyRegistrationParams",
                    "components": [
                        {
                            "name": "pubkeyRegistrationSignature",
                            "type": "tuple",
                            "internalType": "struct BN254.G1Point",
                            "components": [
                                {
                                    "name": "X",
                                    "type": "uint256",
                                    "internalType": "uint256"
                                },
                                {
                                    "name": "Y",
                                    "type": "uint256",
                                    "internalType": "uint256"
                                }
                            ]
                        },
                        {
                            "name": "pubkeyG1",
                            "type": "tuple",
                            "internalType": "struct BN254.G1Point",
                            "components": [
                                {
                                    "name": "X",
                                    "type": "uint256",
                                    "internalType": "uint256"
                                },
                                {
                                    "name": "Y",
                                    "type": "uint256",
                                    "internalType": "uint256"
                                }
                            ]
                        },
                        {
                            "name": "pubkeyG2",
                            "type": "tuple",
                            "internalType": "struct BN254.G2Point",
                            "components": [
                                {
                                    "name": "X",
                                    "type": "uint256[2]",
                                    "internalType": "uint256[2]"
                                },
                                {
                                    "name": "Y",
                                    "type": "uint256[2]",
                                    "internalType": "uint256[2]"
                                }
                            ]
                        }
                    ]
                },
                {
                    "name": "operatorKickParams",
                    "type": "tuple[]",
                    "internalType": "struct IRegistryCoordinator.OperatorKickParam[]",
                    "components": [
                        {
                            "name": "quorumNumber",
                            "type": "uint8",
                            "internalType": "uint8"
                        },
                        {
                            "name": "operator",
                            "type": "address",
                            "internalType": "address"
                        }
                    ]
                },
                {
                    "name": "churnApproverSignature",
                    "type": "tuple",
                    "internalType": "struct ISignatureUtils.SignatureWithSaltAndExpiry",
                    "components": [
                        {
                            "name": "signature",
                            "type": "bytes",
                            "internalType": "bytes"
                        },
                        {
                            "name": "salt",
                            "type": "bytes32",
                            "internalType": "bytes32"
                        },
                        {
                            "name": "expiry",
                            "type": "uint256",
                            "internalType": "uint256"
                        }
                    ]
                },
                {
                    "name": "operatorSignature",
                    "type": "tuple",
                    "internalType": "struct ISignatureUtils.SignatureWithSaltAndExpiry",
                    "components": [
                        {
                            "name": "signature",
                            "type": "bytes",
                            "internalType": "bytes"
                        },
                        {
                            "name": "salt",
                            "type": "bytes32",
                            "internalType": "bytes32"
                        },
                        {
                            "name": "expiry",
                            "type": "uint256",
                            "internalType": "uint256"
                        }
                    ]
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "registries",
            "inputs": [
                {
                    "name": "",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "renounceOwnership",
            "inputs": [],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "serviceManager",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "contract IServiceManager"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "setChurnApprover",
            "inputs": [
                {
                    "name": "_churnApprover",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "setEjector",
            "inputs": [
                {
                    "name": "_ejector",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "setOperatorSetParams",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "operatorSetParams",
                    "type": "tuple",
                    "internalType": "struct IRegistryCoordinator.OperatorSetParam",
                    "components": [
                        {
                            "name": "maxOperatorCount",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "kickBIPsOfOperatorStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        },
                        {
                            "name": "kickBIPsOfTotalStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        }
                    ]
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "setPauserRegistry",
            "inputs": [
                {
                    "name": "newPauserRegistry",
                    "type": "address",
                    "internalType": "contract IPauserRegistry"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "stakeRegistry",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "contract IStakeRegistry"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "transferOwnership",
            "inputs": [
                {
                    "name": "newOwner",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "unpause",
            "inputs": [
                {
                    "name": "newPausedStatus",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "updateOperators",
            "inputs": [
                {
                    "name": "operators",
                    "type": "address[]",
                    "internalType": "address[]"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "updateOperatorsForQuorum",
            "inputs": [
                {
                    "name": "operatorsPerQuorum",
                    "type": "address[][]",
                    "internalType": "address[][]"
                },
                {
                    "name": "quorumNumbers",
                    "type": "bytes",
                    "internalType": "bytes"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "updateSocket",
            "inputs": [
                {
                    "name": "socket",
                    "type": "string",
                    "internalType": "string"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "event",
            "name": "ChurnApproverUpdated",
            "inputs": [
                {
                    "name": "prevChurnApprover",
                    "type": "address",
                    "indexed": false,
                    "internalType": "address"
                },
                {
                    "name": "newChurnApprover",
                    "type": "address",
                    "indexed": false,
                    "internalType": "address"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "EjectorUpdated",
            "inputs": [
                {
                    "name": "prevEjector",
                    "type": "address",
                    "indexed": false,
                    "internalType": "address"
                },
                {
                    "name": "newEjector",
                    "type": "address",
                    "indexed": false,
                    "internalType": "address"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "Initialized",
            "inputs": [
                {
                    "name": "version",
                    "type": "uint8",
                    "indexed": false,
                    "internalType": "uint8"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "OperatorDeregistered",
            "inputs": [
                {
                    "name": "operator",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                },
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "indexed": true,
                    "internalType": "bytes32"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "OperatorRegistered",
            "inputs": [
                {
                    "name": "operator",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                },
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "indexed": true,
                    "internalType": "bytes32"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "OperatorSetParamsUpdated",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "indexed": true,
                    "internalType": "uint8"
                },
                {
                    "name": "operatorSetParams",
                    "type": "tuple",
                    "indexed": false,
                    "internalType": "struct IRegistryCoordinator.OperatorSetParam",
                    "components": [
                        {
                            "name": "maxOperatorCount",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "kickBIPsOfOperatorStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        },
                        {
                            "name": "kickBIPsOfTotalStake",
                            "type": "uint16",
                            "internalType": "uint16"
                        }
                    ]
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "OperatorSocketUpdate",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "indexed": true,
                    "internalType": "bytes32"
                },
                {
                    "name": "socket",
                    "type": "string",
                    "indexed": false,
                    "internalType": "string"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "OwnershipTransferred",
            "inputs": [
                {
                    "name": "previousOwner",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                },
                {
                    "name": "newOwner",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "Paused",
            "inputs": [
                {
                    "name": "account",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                },
                {
                    "name": "newPausedStatus",
                    "type": "uint256",
                    "indexed": false,
                    "internalType": "uint256"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "PauserRegistrySet",
            "inputs": [
                {
                    "name": "pauserRegistry",
                    "type": "address",
                    "indexed": false,
                    "internalType": "contract IPauserRegistry"
                },
                {
                    "name": "newPauserRegistry",
                    "type": "address",
                    "indexed": false,
                    "internalType": "contract IPauserRegistry"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "QuorumBlockNumberUpdated",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "indexed": true,
                    "internalType": "uint8"
                },
                {
                    "name": "blocknumber",
                    "type": "uint256",
                    "indexed": false,
                    "internalType": "uint256"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "Unpaused",
            "inputs": [
                {
                    "name": "account",
                    "type": "address",
                    "indexed": true,
                    "internalType": "address"
                },
                {
                    "name": "newPausedStatus",
                    "type": "uint256",
                    "indexed": false,
                    "internalType": "uint256"
                }
            ],
            "anonymous": false
        }
    ]"#,
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    StakeRegistryAbi,
    r#"[
        {
            "type": "function",
            "name": "WEIGHTING_DIVISOR",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "stateMutability": "pure"
        },
        {
            "type": "function",
            "name": "addStrategies",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "strategyParams",
                    "type": "tuple[]",
                    "internalType": "struct IStakeRegistry.StrategyParams[]",
                    "components": [
                        {
                            "name": "strategy",
                            "type": "address",
                            "internalType": "contract IStrategy"
                        },
                        {
                            "name": "multiplier",
                            "type": "uint96",
                            "internalType": "uint96"
                        }
                    ]
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "delegation",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "contract IDelegationManager"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "deregisterOperator",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "quorumNumbers",
                    "type": "bytes",
                    "internalType": "bytes"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "getCurrentStake",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint96",
                    "internalType": "uint96"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getCurrentTotalStake",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint96",
                    "internalType": "uint96"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getLatestStakeUpdate",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "tuple",
                    "internalType": "struct IStakeRegistry.StakeUpdate",
                    "components": [
                        {
                            "name": "updateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "nextUpdateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "stake",
                            "type": "uint96",
                            "internalType": "uint96"
                        }
                    ]
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getStakeAtBlockNumber",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "blockNumber",
                    "type": "uint32",
                    "internalType": "uint32"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint96",
                    "internalType": "uint96"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getStakeAtBlockNumberAndIndex",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "blockNumber",
                    "type": "uint32",
                    "internalType": "uint32"
                },
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "index",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint96",
                    "internalType": "uint96"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getStakeHistory",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "tuple[]",
                    "internalType": "struct IStakeRegistry.StakeUpdate[]",
                    "components": [
                        {
                            "name": "updateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "nextUpdateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "stake",
                            "type": "uint96",
                            "internalType": "uint96"
                        }
                    ]
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getStakeUpdateAtIndex",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "index",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "tuple",
                    "internalType": "struct IStakeRegistry.StakeUpdate",
                    "components": [
                        {
                            "name": "updateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "nextUpdateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "stake",
                            "type": "uint96",
                            "internalType": "uint96"
                        }
                    ]
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getStakeUpdateIndexAtBlockNumber",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "blockNumber",
                    "type": "uint32",
                    "internalType": "uint32"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint32",
                    "internalType": "uint32"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getTotalStakeAtBlockNumberFromIndex",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "blockNumber",
                    "type": "uint32",
                    "internalType": "uint32"
                },
                {
                    "name": "index",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint96",
                    "internalType": "uint96"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getTotalStakeHistoryLength",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getTotalStakeIndicesAtBlockNumber",
            "inputs": [
                {
                    "name": "blockNumber",
                    "type": "uint32",
                    "internalType": "uint32"
                },
                {
                    "name": "quorumNumbers",
                    "type": "bytes",
                    "internalType": "bytes"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint32[]",
                    "internalType": "uint32[]"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getTotalStakeUpdateAtIndex",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "index",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "tuple",
                    "internalType": "struct IStakeRegistry.StakeUpdate",
                    "components": [
                        {
                            "name": "updateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "nextUpdateBlockNumber",
                            "type": "uint32",
                            "internalType": "uint32"
                        },
                        {
                            "name": "stake",
                            "type": "uint96",
                            "internalType": "uint96"
                        }
                    ]
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "initializeQuorum",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "minimumStake",
                    "type": "uint96",
                    "internalType": "uint96"
                },
                {
                    "name": "strategyParams",
                    "type": "tuple[]",
                    "internalType": "struct IStakeRegistry.StrategyParams[]",
                    "components": [
                        {
                            "name": "strategy",
                            "type": "address",
                            "internalType": "contract IStrategy"
                        },
                        {
                            "name": "multiplier",
                            "type": "uint96",
                            "internalType": "uint96"
                        }
                    ]
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "minimumStakeForQuorum",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint96",
                    "internalType": "uint96"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "modifyStrategyParams",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "strategyIndices",
                    "type": "uint256[]",
                    "internalType": "uint256[]"
                },
                {
                    "name": "newMultipliers",
                    "type": "uint96[]",
                    "internalType": "uint96[]"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "registerOperator",
            "inputs": [
                {
                    "name": "operator",
                    "type": "address",
                    "internalType": "address"
                },
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "quorumNumbers",
                    "type": "bytes",
                    "internalType": "bytes"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint96[]",
                    "internalType": "uint96[]"
                },
                {
                    "name": "",
                    "type": "uint96[]",
                    "internalType": "uint96[]"
                }
            ],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "registryCoordinator",
            "inputs": [],
            "outputs": [
                {
                    "name": "",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "removeStrategies",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "indicesToRemove",
                    "type": "uint256[]",
                    "internalType": "uint256[]"
                }
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "strategyParamsByIndex",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "index",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "tuple",
                    "internalType": "struct IStakeRegistry.StrategyParams",
                    "components": [
                        {
                            "name": "strategy",
                            "type": "address",
                            "internalType": "contract IStrategy"
                        },
                        {
                            "name": "multiplier",
                            "type": "uint96",
                            "internalType": "uint96"
                        }
                    ]
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "strategyParamsLength",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint256",
                    "internalType": "uint256"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "updateOperatorStake",
            "inputs": [
                {
                    "name": "operator",
                    "type": "address",
                    "internalType": "address"
                },
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "internalType": "bytes32"
                },
                {
                    "name": "quorumNumbers",
                    "type": "bytes",
                    "internalType": "bytes"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint192",
                    "internalType": "uint192"
                }
            ],
            "stateMutability": "nonpayable"
        },
        {
            "type": "function",
            "name": "weightOfOperatorForQuorum",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "internalType": "uint8"
                },
                {
                    "name": "operator",
                    "type": "address",
                    "internalType": "address"
                }
            ],
            "outputs": [
                {
                    "name": "",
                    "type": "uint96",
                    "internalType": "uint96"
                }
            ],
            "stateMutability": "view"
        },
        {
            "type": "event",
            "name": "MinimumStakeForQuorumUpdated",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "indexed": true,
                    "internalType": "uint8"
                },
                {
                    "name": "minimumStake",
                    "type": "uint96",
                    "indexed": false,
                    "internalType": "uint96"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "OperatorStakeUpdate",
            "inputs": [
                {
                    "name": "operatorId",
                    "type": "bytes32",
                    "indexed": true,
                    "internalType": "bytes32"
                },
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "indexed": false,
                    "internalType": "uint8"
                },
                {
                    "name": "stake",
                    "type": "uint96",
                    "indexed": false,
                    "internalType": "uint96"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "QuorumCreated",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "indexed": true,
                    "internalType": "uint8"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "StrategyAddedToQuorum",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "indexed": true,
                    "internalType": "uint8"
                },
                {
                    "name": "strategy",
                    "type": "address",
                    "indexed": false,
                    "internalType": "contract IStrategy"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "StrategyMultiplierUpdated",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "indexed": true,
                    "internalType": "uint8"
                },
                {
                    "name": "strategy",
                    "type": "address",
                    "indexed": false,
                    "internalType": "contract IStrategy"
                },
                {
                    "name": "multiplier",
                    "type": "uint256",
                    "indexed": false,
                    "internalType": "uint256"
                }
            ],
            "anonymous": false
        },
        {
            "type": "event",
            "name": "StrategyRemovedFromQuorum",
            "inputs": [
                {
                    "name": "quorumNumber",
                    "type": "uint8",
                    "indexed": true,
                    "internalType": "uint8"
                },
                {
                    "name": "strategy",
                    "type": "address",
                    "indexed": false,
                    "internalType": "contract IStrategy"
                }
            ],
            "anonymous": false
        }
    ]"#,
    event_derives(serde::Deserialize, serde::Serialize)
);
