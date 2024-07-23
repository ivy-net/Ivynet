# Ivynet API Specification (Draft)

<https://api.ivynet.dev/v1/>

## Status Codes

### Success Messages

`200` OK Successful request <br>
`201` Created New object saved <br>
`204` No content Object deleted <br>

### Client Errors

`400` Bad Request Returns JSON with the error message<br>
`401` Unauthorized Couldn't authenticate your request<br>
`402` 2FA Token required Re-try request with user’s 2FA token as CB-2FA-Token header<br>
`403` Invalid scope User hasn't authorized necessary scope<br>
`404` Not Found No such object<br>
`429` Too Many Requests Your connection is being rate limited<br>

### Server Errors

`500` Internal Server Error Something went wrong<br>
`503` Service Unavailable Your connection is being throttled or the service is down for maintenance<br>

# Endpoints

## Client

Endpoints that focus on a specific machine and its deployed AVS. These endpoints will contain health checks, version information for the ivy client as well as the currently deployed AVS, and allow for updates and reconfiguration of the deployed AVS.

### Get Overall System Status

**Endpoint:** `/client/status`

**Method:** `GET`

**Description:** Retrieves the overall status of the system

**Response:**

```json
{
  "error": [],
  "result": {
    "total_machines": 100,
    "healthy_machines": 94,
    "unhealthy_machines": ["machine_id1", "machine_id8", "machine_id45", "machine_id65", "machine_id89"],
    "idle_machines": ["machine_id5"],
    "updateable_machines": ["machine_id65", "machine_id89"],
    "erroring_machines": ["machine_id1", "machine_id8", "machine_id45"],
  }
}
```

### Get All Client Machines

**Endpoint:** `/client/`

**Method:** `GET`

**Description:** Retrieves basic information for each deployed client instance

**Response:**

```json
{
  "error": [],
  "result": {
    "machines": [
        {
            "machine_id": "machine_id3",
            "status": "Healthy",
        }
    ]
  }
}
```

### Get Machine Info

**Endpoint:** `/client/info/{machine_id}`

**Method:** `GET`

**Description:** Retrieves AVS information, health status (including idle and error), and hardware metrics for a specific machine.

**Response:**

```json
{
    "error": [],
    "result": {
        "machine_id": "machine_id3",
        "status": "Healthy",
        "metrics": {
            "cpu_usage": 55.2,
            "memory_usage": 60.4,
            "disk_usage": 70.8,
            "uptime": 123456,
            "deployed_avs": "eigenda",
            "deployed_avs_chain": "mainnet",
            "operator_pub_key": "0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd",
            "errors": [],
            },
        "last_checked": "2024-07-18T12:34:56Z"
        }
}
```

or

```json
{
    "error": [],
    "result": {
        "machine_id": "machine_id8",
        "status": "Error",
        "metrics": {
            "cpu_usage": 55.2,
            "memory_usage": 60.4,
            "disk_usage": 70.8,
            "uptime": 123456,
            "deployed_avs": "eigenda",
            "deployed_avs_chain": "holesky",
            "operator_pub_key": "0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd",
            "errors": ["UNREGISTERED_OPERATOR"],
            },
        "last_checked": "2024-07-18T12:34:56Z"
        }
}
```

### Get All Idle Machines

**Endpoint:** `/client/idle`

**Method:** `GET`

**Description:** Retrieves IDs of idle machines.

**Response:**

```json
{
  "error": [],
  "result": {
    "idle_machines": ["machine_id5"]
  }
}
```

### Get All Unhealthy Machines

**Endpoint:** `/client/unhealthy`

**Method:** `GET`

**Description:** Retrieves IDs of all unhealthy machines which includes machines with a deployed instance that are unregistered onchain, erroring machines, machines needing client updates, and offline machines with AVS registered status.

**Response:**

```json
{
  "error": [],
  "result": {
    "unhealthy_machines": ["machine_id1", "machine_id8", "machine_id45", "machine_id65", "machine_id89"]
  }
}
```

### Get All Erroring Machines

**Endpoint:** `/client/errors`

**Method:** `GET`

**Description:** Retrieves IDs of all machines in serious error

**Response:**

```json
{
  "error": [],
  "result": {
    "erroring_machines": ["machine_id1", "machine_id8", "machine_id45"]
  }
}
```

### Get All Updateable Machines

**Endpoint:** `/client/updateable`

**Method:** `GET`

**Description:** Retrieves IDs of all machines which need an update to their Ivynet client

**Response:**

```json
{
  "error": [],
  "result": {
    "updateable_machines": ["machine_id65", "machine_id89"]
  }
}
```

<!-- TODO: future: update an ivynet client -->
<!-- TODO: future: new avs deployment -->
<!-- TODO: future: update an avs deployment -->

## AVS

AVS Specific information like what AVS's are being run, what AVS's can be run, registration information, etc.

### Get All Deployable AVS's

**Endpoint:** `/avs/`

**Method:** `GET`

**Description:** Retrieves all AVS's deployable with the Ivynet Client

**Response:**

```json
{
  "error": [],
  "result": {
    "deployable_avs": [
        {
            "avs_name": "EigenDA",
            "avs_id": "eigenda",
            "networks": ["mainnet", "holesky"],
        },
        {
            "avs_name": "Lagrange ZK Prover Network",
            "avs_id": "lagrange_zk",
            "networks": ["mainnet", "holesky"],
        },
        {
            "avs_name": "Lagrange State Committees",
            "avs_id": "lagrange_sc",
            "networks": ["mainnet", "holesky"],
        },
        {
            "avs_name": "Cyber MACH",
            "avs_id": "altlayer_mach_cyber",
            "networks": ["mainnet", "holesky"],
        },
        {
            "avs_name": "AltLayer MACH",
            "avs_id": "altlayer_mach",
            "networks": ["mainnet", "holesky"],
        },
        {
            "avs_name": "Xterio MACH",
            "avs_id": "altlayer_mach_xterio",
            "networks": ["mainnet", "holesky"],
        },
        {
            "avs_name": "OpenLayer",
            "avs_id": "openlayer",
            "networks": ["mainnet", "holesky"],
        },
        {
            "avs_name": "incredible_squaring",
            "avs_id": "incredible_squaring",
            "networks": ["local"],
        },
    ]
  }
}
```

### Get All Deployed AVS's

**Endpoint:** `/avs/deployed/`

**Method:** `GET`

**Description:** Retrieves all Ivynet deployed AVS's

**Response:**

```json
{
  "error": [],
  "result": {
    "deployed_avs": [
        {
            "avs_name": "EigenDA",
            "avs_id": "eigenda",
            "network": "mainnet",
            "avs_version": "0.7.4",
            "machine_id": "machine_id1",
            "operator": "0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd",
            "registered": true
        },
        {
            "avs_name": "Lagrange ZK Prover Network",
            "avs_id": "lagrange_zk",
            "network": "holesky",
            "avs_version": "0.4.3",
            "machine_id": "machine_id2",
            "operator": "0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd",
            "registered": false
        },
        // More deployed instances
    ]
  }
}
```

### Get All Registered AVS's

**Endpoint:** `/avs/registered/`

**Method:** `GET`

**Description:** Retrieves all registrations statuses for Ivynet deployable AVS's

**Response:**

```json
{
  "error": [],
  "result": {
    "registered_avs": [
        {
            "avs_name": "EigenDA",
            "avs_id": "eigenda",
            "network": "mainnet",
            "machine_id": "machine_id1",
            "operator": "0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd",
            "registered": true
        },
    ]
  }
}
```

### Get Registration Status

**Endpoint:** `/avs/registered/{avs_id}`

**Method:** `GET`

**Description:** Retrieves registration status for a specific AVS

**Response:**

```json
{
  "error": [],
  "result": true
}
```

### Get AVS Deployments Needing Updates

**Endpoint:** `/avs/updateable/`

**Method:** `GET`

**Description:** Retrieves all Ivynet deployed AVS's that need an update

**Response:**

```json
{
  "error": [],
  "result": {
    "updateable_avs": [
        {
            "avs_id": "eigenda",
            "network": "mainnet",
            "avs_version": "0.6.9",
            "machine_id": "machine_id1",
        },
        // More updateable instances
    ]
  }
}
```

### Get AVS Hardware Requirements

**Endpoint:** `/avs/requirements/{avs_id}&{operator_address}`

**Method:** `GET`

**Description:** Retrieves hardware, bandwidth, stake, and allowlist requirements for a specific AVS tailored to an operator's stake

**Example Request:** /avs/requirements/lagrange_zk&0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd

**Response:**

```json
{
  "error": [],
  "result": {
    "physical": {
        "node_class": "4XL",
        "cores": 16,
        "memory": 180000000000, //180 gb
        "disk": 60000000000, //60gb
        "bandwidth": 5000000000, //5gbps - not specified by lagrange, taken from EigenLayer node classes
    },
    "stake": {}, //TODO: Quorum stuff - unsure about breakdown
    "allowlist": true,
  }
}
```

### Get AVS Rewards Rate

**Endpoint:** `/avs/rewards/{avs_id}&{currency}&{operator_address}`

**Method:** `GET`

**Description:** Retrieves monthly payment for validating a specific AVS in dollar and eth basis based on operator stake

**Example Request:** /avs/rewards/eigenda&0x0a3e3d83c99b27ca7540720b54105c79cd58dbdd

**Response:**

```json
{
  "error": [],
  "result": {
    "dollar": 1657.33,
    "ethereum": 0.50885
  }
}
```

<!-- TODO: future: avs historical rewards -->
<!-- TODO: future: avs historical slashing risk -->

## Operator

Operator specific information like information about their delegation, currently registered avs count,

### Get AVS Rewards Rate

**Endpoint:** `/operator/stake/{operator_address}`

**Method:** `GET`

**Description:** Retrieves delegated stake for each quorum on EigenLayer

**Response:**

```json
{
  "error": [],
  "result": {
    "eigenlayer": {
        "eigen": 0,
        "ethereum": {[
            {
                "name": "Cbeth",
                "address": "0x54945180dB7943c0ed0FEE7EdaB2Bd24620256bc",
                "delegated_stake": 351
            },
            {
                "name": "Steth",
                "address": "0x93c4b944D05dfe6df7645A86cd2206016c51564D",
                "delegated_stake": 732
            },
            {
                "name": "Reth",
                "address": "0x1BeE69b7dFFfA4E2d53C2a2Df135C388AD25dCD2",
                "delegated_stake": 923
            },
            {
                "name": "Sweth",
                "address": "0x0Fe4F44beE93503346A3Ac9EE5A26b130a5796d6",
                "delegated_stake": 953
            },
            {
                "name": "Lseth",
                "address": "0xAe60d8180437b5C34bB956822ac2710972584473",
                "delegated_stake": 155
            },
            {
                "name": "Sfrxeth",
                "address": "0x8CA7A5d6f3acd3A7A8bC468a8CD0FB14B6BD28b6",
                "delegated_stake": 169
            },
            {
                "name": "Wbeth",
                "address": "0x7CA911E83dabf90C90dD3De5411a10F1A6112184",
                "delegated_stake": 234
            },
            {
                "name": "Ethx",
                "address": "0x9d7eD45EE2E8FC5482fa2428f15C971e6369011d",
                "delegated_stake": 604
            },
            {
                "name": "Oseth",
                "address": "0x57ba429517c3473B6d34CA9aCd56c0e735b94c02",
                "delegated_stake": 130
            },
            {
                "name": "Meth",
                "address": "0x298aFB19A105D59E74658C4C334Ff360BadE6dd2",
                "delegated_stake": 705
            },
            {
                "name": "Ankreth",
                "address": "0x13760F50a9d7377e4F20CB8CF9e4c26586c658ff",
                "delegated_stake": 286
            },
            {
                "name": "Oeth",
                "address": "0xa4C637e0F704745D182e4D38cAb7E7485321d059",
                "delegated_stake": 657
            },
            {
                "name": "BeaconEth",
                "address": "0xbeaC0eeEeeeeEEeEeEEEEeeEEeEeeeEeeEEBEaC0",
                "delegated_stake": 146
            }
        ]}
    }
  }
}
```

<!-- TODO: registered avs count -->
