use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub node: NodeConfig,
    pub network: NetworkConfig
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub logger: bool,

    pub test_convergence: bool,
    pub test_single: bool,
    pub test_flow: bool,
    pub test_throughput: bool,
    pub test_latency: bool,

    pub sensor_application: bool
}

#[derive(Debug, Deserialize)]
pub struct NodeConfig {
    pub router: bool,
    pub networks: Vec<u16>,
    pub nid: u64,
    pub name: String
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    pub MTU: u32,

    pub ND_RTO_MS: u64,
    pub ND_RETRANSMIT_LIMIT: u64,
    pub ND_TTL_S: u64,
    pub ND_CACHE_SIZE: usize,
    pub DNS_TTL_S: u8,

    pub AD_HOC_TIMEOUT_MS: u64,
    pub AD_HOC_RTO_NS: u64,
    pub AD_HOC_TTL_S: u8,
    pub AD_MAX_HOPS: u8
}