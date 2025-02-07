pub enum Alert {
    Custom = 1,
    ActiveSetNoDeployment = 2,
    CrashedNode = 3,
    HardwareResourceUsage = 4,
    LowPerformanceScore = 5,
    NeedsUpdate = 6,
    NoChainInfo = 7,
    NodeNotRunning = 8,
    NoMetrics = 9,
    NoOperatorId = 10,
    UnregisteredFromActiveSet = 11,
}

// 1. impl errors recognize on ingress
// 2. impl errors based on tracked state

Backend
// 1. Get curret err
// 2. Get historical err
// 3. Post req for err acknowledgement
// 4. resolution is a TODO!()
// 5. IgnoredErrorType table per org
