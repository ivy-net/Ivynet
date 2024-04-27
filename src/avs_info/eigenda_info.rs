use ethers_contract::abigen;

use crate::rpc_management::{self, Network};

lazy_static::lazy_static! {
    pub static ref NETWORK: Network = rpc_management::NETWORK.lock().unwrap().clone();
    pub static ref STAKE_REGISTRY_ADDRESS: String = get_stake_registry_address();
}

pub fn get_stake_registry_address() -> String {
    match NETWORK.clone() {
        Network::Mainnet => "0x006124ae7976137266feebfb3f4d2be4c073139d".to_string(),
        Network::Holesky => "0xBDACD5998989Eec814ac7A0f0f6596088AA2a270".to_string(),
        Network::Local => todo!(),
    }
}

abigen!(
    EigendaStakeRegistryAbi,
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
