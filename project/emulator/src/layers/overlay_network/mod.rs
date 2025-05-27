use std::{net::{Ipv6Addr, SocketAddr}, sync::{Arc, Mutex}};
use once_cell::sync::Lazy;
use overlay_handlers::{handle_destination_fqdn, handle_destination_ilv, handle_destination_nid, handle_ilnp_multicast_buffer, handle_ilnp_unicast_buffer, handle_path_discovery/*, handle_ilnp_buffer, handle_path_discovery*/};
use tokio::{signal, sync::{mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, Mutex as TokioMutex}};
use ttl_cache::TtlCache;
use bytes::BytesMut;

use crate::{
    models::{config_models::Config, network_models::EmulatorSocket, network_packets::INLPv6Packet, protocol_control_block::ILNP_PCB_S}, 
    services::{config_services::get_config, log_services::{log_error, log_info}, network_services::{get_over_interface_by_locator, get_over_interface_by_name, get_over_interfaces, get_over_locators, lookup_forwarding_table_route}, time_services::get_current_timestamp}, 
    layers::underlay_network::{close_underlay_socket, open_underlay_socket, underlay_uni_tx}
};

mod jcmp_tx;
mod overlay_handlers;

/// Node configurations 
///     - load config once to increase performance
///     - globally accessible
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    get_config().expect("Failed to load configuration")
});

/// Protocol Control Block 
///     - used to measure node performance
///     - globally accessible
pub static PCB: Lazy<Mutex<ILNP_PCB_S>> = Lazy::new(|| Mutex::new(ILNP_PCB_S::default()));

/// Address Resolution Table (Neighbour Discovery)
///     - maps NID to (interface, IPv6, Unicast Port)
///     - equivalent of ARP table
static NID_ADDRESS_RESOLUTION_TABLE: Lazy<Mutex<TtlCache<u64, (String, Ipv6Addr, u16)>>> = Lazy::new(|| { Mutex::new(TtlCache::new(CONFIG.network.ND_CACHE_SIZE)) });

/// Name Resolution Table (DNS)
///     - maps HashKey to (FQDN, NID, L64)
///     - maps HashKey to (NID, L64)
///     - HashKey is created using (NID, L64)
///     - each entry is uniquely identifiable by the (NID, L64)
pub static NAME_ILV_TABLE: Lazy<Mutex<TtlCache<u64, (String, u64, u64)>>> = Lazy::new(|| { Mutex::new(TtlCache::new(CONFIG.network.ND_CACHE_SIZE)) });
pub static NID_ILV_TABLE: Lazy<Mutex<TtlCache<u64, (u64, u64)>>> = Lazy::new(|| { Mutex::new(TtlCache::new(CONFIG.network.ND_CACHE_SIZE)) });

/// Forwarding Table
///     - maps HashKey to (next_hop (NID), target locator (L64), next_hop (interface), hop_count)
///     - HashKey is created using next_hop (NID) and target locator (L64)
///     - Each entry is uniquely identifiable by the next hop's identifier (NID) and the target locator (L64)
pub static LOCATOR_FORWARDING_TABLE: Lazy<Mutex<TtlCache<u64, (u64, u64, String, u8)>>> = Lazy::new(|| { Mutex::new(TtlCache::new(CONFIG.network.ND_CACHE_SIZE)) });

/// ILNP data packet queue
///     - required to consume the unicast UDP packets as quick as possible to avoid drops
pub static ILNP_QUEUE: Lazy<(UnboundedSender<(BytesMut, usize, SocketAddr)>, Arc<TokioMutex<UnboundedReceiver<(BytesMut, usize, SocketAddr)>>>)> = Lazy::new(|| {
    let (tx, rx) = unbounded_channel();
    (tx, Arc::new(TokioMutex::new(rx)))
});


/// Open Socket
///     - opening socket at the ILNP layer
///     - thread created to consume and handle JCMP multicast packets
///     - thread created to consume JTP unicast packets
///     - thread created to handle JTP unicast packets from a queue
pub async fn open_ilnp_socket() 
    -> Result<EmulatorSocket, String>
{

    // protocol started - recording time for analysis
    let start_time = get_current_timestamp()?;
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.start_time = start_time;
        },
        Err(_) => {}
    }

    // create socket and clone it
    let emulator_socket = open_underlay_socket().await?;
    let emulator_socket_clone = EmulatorSocket {
        mulcast_socket: emulator_socket.mulcast_socket.clone(),
        unicast_socket: emulator_socket.unicast_socket.clone(),
        local_network: emulator_socket.local_network.clone()
    };
    let emulator_socket_clone2 = EmulatorSocket {
        mulcast_socket: emulator_socket.mulcast_socket.clone(),
        unicast_socket: emulator_socket.unicast_socket.clone(),
        local_network: emulator_socket.local_network.clone()
    };
    let emulator_socket_clone3 = EmulatorSocket {
        mulcast_socket: emulator_socket.mulcast_socket.clone(),
        unicast_socket: emulator_socket.unicast_socket.clone(),
        local_network: emulator_socket.local_network.clone()
    };

    // create async handler to receive multicast packets
    // stop after ctrl+c
    tokio::spawn(async move {

        // get locators we are connected to
        match get_over_locators() {
            Ok(connected_locators) => {

                // more than enough for JCMP packets
                let mut buf = [0; 1024];

                loop {

                    // wait to either receive or ctrl+c
                    tokio::select! {

                        // receive JCMP packet
                        result = emulator_socket_clone.mulcast_socket.recv_from(&mut buf) => {
                            match result {
                                Ok((len, addr)) => {

                                    // clone
                                    let emulator_socket_clone3 = EmulatorSocket {
                                        mulcast_socket: emulator_socket_clone.mulcast_socket.clone(),
                                        unicast_socket: emulator_socket_clone.unicast_socket.clone(),
                                        local_network: emulator_socket_clone.local_network.clone()
                                    };
                                    let connected_locators_clone = connected_locators.clone();
                                    let buf_clone = buf.clone();

                                    // create a new thread to handle the request
                                    // this will free up the reciever again
                                    tokio::spawn(async move {
                                        handle_ilnp_multicast_buffer(&emulator_socket_clone3, &connected_locators_clone, &buf_clone, len, addr).await;
                                    });

                                },
                                Err(err) => {
                                    log_error(&emulator_socket_clone, &format!("open_ilnp_socket(): error receiving a packet: {}", &err.to_string())).await;
                                }
                            }
                        },
                        _ = signal::ctrl_c() => {
                            log_info(&emulator_socket_clone, "open_ilnp_socket(): ctrl+c received, exiting the ilnp receiver handler").await;
                            break;
                        },
                    }
                }

            },
            Err(err) => {
                log_error(&emulator_socket_clone, &err).await;
            }
        }
    });

    // create async handler to receive unicast packets
    // stop after ctrl+c
    tokio::spawn(async move {

        // MTU=1500 if Config.MTU=1412
        let total_mtu = 40 + 8 + 40 + CONFIG.network.MTU;

        // https://docs.rs/bytes/latest/bytes/index.html
        let mut buf = BytesMut::with_capacity(total_mtu as usize);
        let ilnp_tx = &ILNP_QUEUE.0;

        loop {

            // resize to MTU
            buf.resize(total_mtu as usize, 0);

            // wait to either receive or ctrl+c
            tokio::select! {

                // receive JTP packet
                result = emulator_socket_clone2.unicast_socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, addr)) => {

                            // to reduce memory space buffer is reduced to packet size
                            buf.truncate(len);
                            let packet = buf.clone();

                            // packet inserted into the ILNP queue for processing
                            match ilnp_tx.send((packet, len, addr)) {
                                Ok(()) => {},
                                Err(err) => {
                                    log_error(&emulator_socket_clone2, &format!("open_ilnp_socket(): error adding to ilnp queue: {}", err)).await;
                                }
                            }

                            // replace buffer content with 0
                            buf.clear();

                        },
                        Err(err) => {
                            log_error(&emulator_socket_clone2, &format!("open_ilnp_socket(): error receiving a packet: {}", &err.to_string())).await;
                        }
                    }
                },
                _ = signal::ctrl_c() => {
                    log_info(&emulator_socket_clone2, "open_ilnp_socket(): ctrl+c received, exiting the ilnp receiver handler").await;
                    break;
                },
            }
        }
    });

    // create async handler to process the unicast packets
    tokio::spawn(async move {

        // lock the receiver while using it
        // only this function using it so lock indefinitly
        let mut ilnp_rx = ILNP_QUEUE.1.lock().await;

        // loop to consume the queue and handle the packets
        while let Some((buf, len, addr)) = ilnp_rx.recv().await {
            handle_ilnp_unicast_buffer(&emulator_socket_clone3, &buf.as_ref(), len, addr).await;
        }
    });

    // signal that node is up and running as expected
    log_info(&emulator_socket, "open_ilnp_socket(): ILNP layer running.").await;

    // logging time for when protocol is ready to use
    let ready_time = get_current_timestamp()?;
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.ready_time = ready_time;
        },
        Err(_) => {}
    }

    Ok(emulator_socket)
}


/// Close Socket
///     - send the node's PCB to the logger
///     - close the underlay socket
pub async fn close_ilnp_socket(emulator_socket: EmulatorSocket) 
    -> Result<(), String>
{

    // measure finishing time for analysis
    let finish_time = get_current_timestamp()?;
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.finish_time = finish_time;
        },
        Err(_) => {}
    }

    // send the pcb to the logger
    // note PCB;NB corresponds to the topology or number of routers we are testing with
    // this was necessary to distinguish the entries in the logs
    match PCB.lock() {
        Ok(pcb) => {
            match pcb.to_json_string() {
                Ok(json_string) => {
                    log_info(&emulator_socket, &format!("PCB;9;{}", json_string)).await;
                },
                Err(_) => {}
            }
        },
        Err(_) => {}
    }

    // close underlay socket
    close_underlay_socket(emulator_socket).await
}


/// TX Unicast using NID
///     - try to send locally
///     - try to forward the packet
pub async fn ilnp_nid_tx(emulator_socket: &EmulatorSocket, destination_nid:&u64, buf:&[u8])
    -> Result<(), String>
{

    // next hop - Ipv6, port, source locator, destination nid, destination locator
    let mut result: (Ipv6Addr, u16, u64, u64, u64) = (Ipv6Addr::UNSPECIFIED, 0, 0, 0, 0);

    // loop through interfaces to perform address resolution
    match get_over_interfaces() {
        Ok(interfaces) => {
            for (interface_name, (source_locator, _)) in interfaces {

                // address resolution to discover the node
                match handle_destination_nid(emulator_socket, destination_nid, &interface_name).await {
                    Ok((destination_address, destination_port)) => {
                        result = (destination_address, destination_port, source_locator, destination_nid.clone(), source_locator);
                        break;
                    },
                    Err(_) => {}
                }

            }
        },
        Err(err) => {
            return Err(err);
        }
    }

    // if the next hop was not found
    if result.0 == Ipv6Addr::UNSPECIFIED {

        // name resolution
        match handle_destination_ilv(emulator_socket, destination_nid).await {
            Ok(dns_entries) => {

                // first check if path already discovered to avoid storming the network with router requests
                for (_, destination_locator) in &dns_entries {
                    match lookup_forwarding_table_route(&destination_locator) {
                        Ok((router_nid, _, interface_name, _)) => {

                            // retrieve the interface name for the result
                            match get_over_interface_by_name(&interface_name) {
                                Ok((source_locator, _)) => {

                                    // address resolution for the router
                                    match handle_destination_nid(emulator_socket, &router_nid, &interface_name).await {
                                        Ok((router_ipv6, router_port)) => {
                                            result = (router_ipv6, router_port, source_locator, destination_nid.clone(), destination_locator.clone());
                                        },
                                        Err(err) => {
                                            return Err(err);
                                        }
                                    }

                                },
                                Err(err) => {
                                    return Err(err);
                                }
                            }

                        },
                        Err(_) => {}
                    }

                }

                // next hop wasn't found in the forwarding table
                // need to perform path discovery
                if result.0 == Ipv6Addr::UNSPECIFIED {

                    // loop for each (NID, L64) binding received by DNS, ILV
                    // this will happen if node is multi-homed
                    for (_, destination_locator) in &dns_entries {

                        // path discovery
                        match handle_path_discovery(emulator_socket, None, &destination_locator, &0).await {
                            Ok((router_nid, _,  interface_name,  _)) => {
                            
                                match get_over_interface_by_name(&interface_name) {
                                    Ok((source_locator, _)) => {

                                        // address resolution for next hop
                                        match  handle_destination_nid(emulator_socket, &router_nid, &interface_name).await {
                                            Ok((router_ipv6, router_port)) => {

                                                result = (router_ipv6, router_port, source_locator, destination_nid.clone(), destination_locator.clone());

                                            },
                                            Err(err) => {
                                                return Err(err);
                                            }
                                        }

                                    },
                                    Err(err) => {
                                        return Err(err);
                                    }
                                }
                            },
                            Err(_) => {}
                        }

                    }
                }

            },
            Err(err) => {
                return Err(err);
            }
        }

    }

    // host could not be resolved
    if result.0 == Ipv6Addr::UNSPECIFIED {
        return Err("ilnp_nid_tx(): couldn't resolve host".to_string());
    }

    // create the ILNPv6 header
    let inlp_pck = INLPv6Packet::new()
        .with_version(6)
        .with_traffic_class(0)
        .with_flow_label(0)
        .with_payload_length(buf.len() as u16)
        .with_next_header(151)
        .with_hop_limit(1)
        .with_source_locator(result.2)
        .with_source_identifier(emulator_socket.local_network.local_nid)
        .with_destination_locator(result.4)
        .with_destination_identifier(result.3)
        .into_bytes();
    let mut pck_vec: Vec<u8> = inlp_pck.to_vec();
    pck_vec.extend_from_slice(&buf);

    // send the ILNP packet to the underlay network for sending over unicast
    let _ = underlay_uni_tx(emulator_socket, &result.0, &result.1, &pck_vec).await?;

    // count data packets sent
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.data_request_tx += 1;
        },
        Err(_) => {}
    }

    Ok(())
    
}

/// TX Unicast using FQDN
///     - get ILV using FQDN
///     - try to send locally
///     - try to forward packet
pub async fn ilnp_fqdn_tx(emulator_socket: &EmulatorSocket, destination_fqdn:&String, buf:&[u8])
    -> Result<(), String>
{
    // get ILV for FQDN
    let dns_entries = handle_destination_fqdn(emulator_socket, destination_fqdn).await?;

    // loop through each DNS entry
    let mut result: (Ipv6Addr, u16, u64, u64, u64) = (Ipv6Addr::UNSPECIFIED, 0, 0, 0, 0);
    for (destination_nid, destination_locator) in &dns_entries {

        // check if we are connected to the node's locator
        match get_over_interface_by_locator(&destination_locator) {
            Ok(interface_name) => {

                // address resolution
                match handle_destination_nid(emulator_socket, &destination_nid, &interface_name).await {
                    Ok((ipv6, port)) => {
                        result = (ipv6, port, destination_locator.clone(), destination_nid.clone(), destination_locator.clone());
                        break;
                    },
                    Err(_) => {}
                }
            },
            Err(_) => {}
        }

    }

    // if not in our network
    if result.0 == Ipv6Addr::UNSPECIFIED {

        // loop through DNS entries
        for (destination_nid, destination_locator) in &dns_entries {

            // check if we've already resolved the route
            match lookup_forwarding_table_route(&destination_locator) {
                Ok((router_nid, _, interface_name, _)) => {

                    match get_over_interface_by_name(&interface_name) {
                        Ok((source_locator, _))  => {

                            // address resolution to the router
                            match handle_destination_nid(emulator_socket, &router_nid, &interface_name).await {
                                Ok((router_ipv6, router_port)) => { 

                                    result = (router_ipv6, router_port, source_locator, destination_nid.clone(), destination_locator.clone());
                                    break;

                                },
                                Err(err) => {
                                    return Err(err);
                                }
                            }
                        },
                        Err(err) => { 
                            return Err(err);
                        }
                    }
                },
                Err(_) => {}
            }

        }
    }
    
    // if we haven't already resolved a route for the locator
    if result.0 == Ipv6Addr::UNSPECIFIED {

        // loop through DNS entries
        for (destination_nid, destination_locator) in &dns_entries {

            // measure starting time of path discovery
            // this is not essential for the protocol
            // the number here needs to be changed as we increase the number of routers in our analysis
            if CONFIG.app.test_convergence {
                log_info(emulator_socket, "DISCOVERY_STARTED;1").await;
            }

            // path discovery
            match handle_path_discovery(emulator_socket, None, &destination_locator, &0).await {
                Ok((router_nid, _, interface_name, _)) => {

                    // measure ending time of path discovery
                    // the number here needs to be changed as we increase the number of routers in our analysis
                    if CONFIG.app.test_convergence {
                        log_info(emulator_socket, "DISCOVERY_COMPLETED;1").await;
                    }

                    match get_over_interface_by_name(&interface_name) {
                        Ok((source_locator, _))  => {

                            // address resolution for next hop
                            match handle_destination_nid(emulator_socket, &router_nid, &interface_name).await {
                                Ok((router_ipv6, router_port)) => {

                                    result = (router_ipv6, router_port, source_locator, destination_nid.clone(), destination_locator.clone());

                                },
                                Err(err) => { 
                                    return Err(err);
                                }
                            }

                        },
                        Err(err) => {
                            return Err(err);
                        }
                    }

                },
                Err(_) => {}
            }
        }
    }

    // could not resolve host
    if result.0 == Ipv6Addr::UNSPECIFIED {
        return Err("ilnp_fqdn_tx(): couldn't resolve host".to_string());
    }

    // create the ILNPv6 header
    let inlp_pck = INLPv6Packet::new()
        .with_version(6)
        .with_traffic_class(0)
        .with_flow_label(0)
        .with_payload_length(buf.len() as u16)
        .with_next_header(151)
        .with_hop_limit(1)
        .with_source_locator(result.2)
        .with_source_identifier(emulator_socket.local_network.local_nid)
        .with_destination_locator(result.4)
        .with_destination_identifier(result.3)
        .into_bytes();

    let mut pck_vec: Vec<u8> = inlp_pck.to_vec();
    pck_vec.extend_from_slice(&buf);

    // send the ILNP packet to the underlay network for sending over unicast
    let _ = underlay_uni_tx(emulator_socket, &result.0, &result.1, &pck_vec).await?;

    // count the data packets
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.data_request_tx += 1;
        },
        Err(_) => {}
    }

    Ok(())

}