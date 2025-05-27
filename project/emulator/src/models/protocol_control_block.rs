#![allow(non_camel_case_types)]

use serde::Serialize;
use serde_json;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ILNP_PCB_S {
    
    // start / end
    pub start_time: u64,
    pub ready_time: u64,
    pub finish_time: u64,

    // jtp packets
    pub data_request_rx: u64,
    pub data_request_tx: u64,
    pub data_request_forward_rx: u64,
    pub data_request_forward_tx: u64,

    // jcmp neighbour discovery
    pub nd_solicitation_jcmp_rx: u64,
    pub nd_solicitation_jcmp_tx: u64,
    pub nd_advertisement_jcmp_rx: u64,
    pub nd_advertisement_jcmp_tx: u64,

    // jcmp dns fqdn lookup
    pub dns_fqdn_query_jcmp_rx: u64,
    pub dns_fqdn_query_jcmp_tx: u64,
    pub dns_fqdn_response_jcmp_rx: u64,
    pub dns_fqdn_response_jcmp_tx: u64,

    // jcmp dns ilv lookup
    pub dns_ilv_query_jcmp_rx: u64,
    pub dns_ilv_query_jcmp_tx: u64,
    pub dns_ilv_response_jcmp_rx: u64,
    pub dns_ilv_response_jcmp_tx: u64,

    // jcmp path discovery
    pub router_request_jcmp_rx: u64,
    pub router_request_jcmp_tx: u64,
    pub router_response_jcmp_rx: u64,
    pub router_response_jcmp_tx: u64

}

impl ILNP_PCB_S {
    pub fn default() 
        -> Self 
    {
        Self {
            start_time: 0,
            ready_time: 0,
            finish_time: 0,
            data_request_rx: 0,
            data_request_tx: 0,
            data_request_forward_rx: 0,
            data_request_forward_tx: 0,
            nd_solicitation_jcmp_rx: 0,
            nd_solicitation_jcmp_tx: 0,
            nd_advertisement_jcmp_rx: 0,
            nd_advertisement_jcmp_tx: 0,
            dns_fqdn_query_jcmp_rx: 0,
            dns_fqdn_query_jcmp_tx: 0,
            dns_fqdn_response_jcmp_rx: 0,
            dns_fqdn_response_jcmp_tx: 0,
            dns_ilv_query_jcmp_rx: 0,
            dns_ilv_query_jcmp_tx: 0,
            dns_ilv_response_jcmp_rx: 0,
            dns_ilv_response_jcmp_tx: 0,
            router_request_jcmp_rx: 0,
            router_request_jcmp_tx: 0,
            router_response_jcmp_rx: 0,
            router_response_jcmp_tx: 0
        }
    }

    pub fn to_json_string(&self) 
        ->  Result<String, String> 
    {
        match serde_json::to_string(self) {
            Ok(json_string)  => {
                Ok(json_string)
            },
            Err(err) => {
                Err(format!("ILNP_PCB_S::to_json_string(): failed to serialise PCB: {}", err))
            }
        }
    }
}