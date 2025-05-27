use std::{net::Ipv6Addr, sync::Arc};
use tokio::net::UdpSocket;

#[derive(Debug, Clone)]
pub struct EmulatorLocalNetwork {
    pub local_uid: u16,
    pub local_index: u32,
    pub local_nid: u64,
    pub local_fqdn: String,
    #[allow(dead_code)]
    pub local_ipv6: Ipv6Addr,
    pub local_port: u16
}
impl EmulatorLocalNetwork  
{
    pub fn set_local_port(&mut self, new_port: u16) {
        self.local_port = new_port;
    }
}

#[derive(Debug, Clone)]
pub struct EmulatorSocket {
    pub mulcast_socket: Arc<UdpSocket>,
    pub unicast_socket: Arc<UdpSocket>,
    pub local_network: EmulatorLocalNetwork
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct JTPResponse {
    pub source_locator: u64,
    pub source_nid: u64,
    pub destination_locator: u64,
    pub destination_nid: u64,
    pub payload: Vec<u8>
}