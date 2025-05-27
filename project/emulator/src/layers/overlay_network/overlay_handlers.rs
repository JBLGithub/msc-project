use std::{net::{IpAddr, Ipv6Addr, SocketAddr}, thread, time::Duration};
use std::convert::TryInto;

use tokio::time::Instant;

use crate::{layers::{jtp_network::JTP_QUEUE, underlay_network::underlay_uni_tx}, models::{network_models::{EmulatorSocket, JTPResponse}, network_packets::{INLPv6Packet, JCMP_DNS_FQDN_Query_Packet, JCMP_DNS_FQDN_Response_Packet, JCMP_DNS_ILV_Response_Packet, JCMP_ND_Advertisement, JCMP_Router_Request, JCMP_Router_Response}}, services::{log_services::log_error, network_services::{get_over_interface_by_locator, get_over_interfaces, insert_into_forwarding_table, insert_into_name_ilv_table, insert_into_nid_ilv_table, lookup_forwarding_table, lookup_forwarding_table_route, lookup_name_ilv_table, lookup_nid_ilv_table}}};
use super::{jcmp_tx::{jcmp_tx_advertisement, jcmp_tx_dns_fqdn_query, jcmp_tx_dns_fqdn_response, jcmp_tx_dns_ilv_query, jcmp_tx_dns_ilv_response, jcmp_tx_router_request, jcmp_tx_router_response, jcmp_tx_solicitation}, CONFIG, NID_ADDRESS_RESOLUTION_TABLE, PCB};


/// Handler for the JCMP multicast receiver
///     - parses the socket, connected locators, buffer, buffer length, source IPv6 address
///     - filter packets if running on the same machine
pub async fn handle_ilnp_multicast_buffer(emulator_socket: &EmulatorSocket, connected_locators: &Vec<u64>, buf: &[u8], len: usize, addr: SocketAddr)
{

    // extract source IPv6
    match addr.ip() {
        IpAddr::V4(_) => {
            log_error(&emulator_socket, "handle_ilnp_multicast_buffer(): received invalid packet: ipv4 packet").await;
            return;
        },
        IpAddr::V6(source_address) => {

            // check packet has ILNP header
            if len < 40 {
                //log_error(&emulator_socket, "handle_ilnp_multicast_buffer(): received invalid multicast packet: packet too small").await;
                return;
            }

            // extract ilnp header
            match buf[0..40].try_into() as Result<[u8; 40], _> {
                Ok(ilnp_header) => {
        
                    // parse the header into the struct
                    let ilnp_pck = INLPv6Packet::from_bytes(ilnp_header);

                    // check the Next Header byte to check it's a JCMP packet
                    if ilnp_pck.next_header() == 150 {
                    
                        // because of IPV6_MULTICAST_LOOP and running instances on the same machine
                        // filter packets using locators we're supposed to be connected to
                        // not required when running on different machines
                        if connected_locators.contains(&ilnp_pck.destination_locator()) {
        
                            // check if packet code (in JCMP header) is included
                            if len < 41 {
                                log_error(&emulator_socket, "handle_ilnp_multicast_buffer(): received invalid jcmp packet: missing code").await;
                                return;
                            }
            
                            // send to JCMP handler
                            let payload = &buf[40..len];
                            handle_jcmp_packet(emulator_socket, source_address, ilnp_pck, payload).await;

                        }
        
                    }
                            
                    else {
                        // log packets are being received here - causing infifinite loop
                        // log_error(&emulator_socket, "handle_ilnp_multicast_buffer(): received invalid packet: wrong header or type").await;
                        return;
                    }
        
                },
                Err(err) => {
                    log_error(&emulator_socket, &format!("handle_ilnp_multicast_buffer(): failed to parse ilnp header: {}", err)).await;
                }
            }

        }
    }

}


/// Handler for the JTP unicast receiver
pub async fn handle_ilnp_unicast_buffer(emulator_socket: &EmulatorSocket, buf: &[u8], len: usize, addr: SocketAddr)
{

    // extract source IPv6
    match addr.ip() {
        IpAddr::V4(_) => {
            log_error(&emulator_socket, "handle_ilnp_unicast_buffer(): received ipv4 packet").await;
            return;
        },
        IpAddr::V6(_) => {
    
            // check packet has ilnp header
            if len < 40 {
                log_error(&emulator_socket, "handle_ilnp_unicast_buffer(): received invalid packet: packet too small").await;
                return;
            }

            // extract ilnp header
            match buf[0..40].try_into() as Result<[u8; 40], _> {
                Ok(ilnp_header) => {

                    // parse header into a struct
                    let ilnp_pck = INLPv6Packet::from_bytes(ilnp_header);    

                    // check Next.Header is 151 for JTP packets
                    if ilnp_pck.next_header() == 151 {

                        // unicast payload
                        let payload = &buf[40..len];

                        // check the packet is for us
                        if ilnp_pck.destination_identifier() == emulator_socket.local_network.local_nid {

                            // count packet received
                            let mut err: String = "".to_string();
                            match PCB.lock() {
                                Ok(mut pcb) => {
                                    pcb.data_request_rx += 1;
                                },
                                Err(error) => {
                                    err = error.to_string();
                                }
                            }
                            if err != "" {
                                log_error(emulator_socket, "handle_ilnp_unicast_buffer(): failed to lock PCB").await;
                            }

                            // send the packet to the user
                            let jtp_receive = JTPResponse {
                                source_locator: ilnp_pck.source_locator(),
                                source_nid: ilnp_pck.source_identifier(),
                                destination_locator: ilnp_pck.destination_locator(),
                                destination_nid: ilnp_pck.destination_identifier(),
                                payload: payload.to_vec()
                            };
                            let tx = &JTP_QUEUE.0;
                            if let Err(/*err*/_) = tx.try_send(jtp_receive) {
                                //log_error(&emulator_socket, &format!("handle_ilnp_unicast_buffer(): failed to add ilnp buffer to JTP queue: {}", err)).await;
                            }

                        }
                                
                        // not intended for us
                        // forwarding packet only if we are a router
                        else if CONFIG.node.router {

                            // handler to forward packets
                            match handle_router_forward(&emulator_socket, &ilnp_pck, &payload).await
                            {
                                Ok(()) => {

                                    // count forwarding packets
                                    match PCB.lock() {
                                        Ok(mut pcb) => {
                                            pcb.data_request_forward_rx += 1;
                                            pcb.data_request_forward_tx += 1;
                                        },
                                        Err(_) => {}
                                    }

                                    //log_info(&emulator_socket, "handle_router_forward(): successufully forwarded packet").await;
                                },
                                Err(err) => {
                                    log_error(&emulator_socket, &err).await;
                                }
                            }

                        }

                        else {
                            log_error(&emulator_socket, "handle_ilnp_unicast_buffer(): received packet not intended for us").await;
                            return;
                        }

                    } else {
                        log_error(&emulator_socket, "handle_ilnp_unicast_buffer(): received invalid packet: wrong header or type").await;
                        return;
                    }

                },
                Err(err) => {
                    log_error(&emulator_socket, &format!("handle_ilnp_unicast_buffer(): failed to parse ilnp header: {}", err)).await;
                }
            }
        }
    }

}


/// Handles the different types of JCMP packets
///     - Packet Code 0     (Neigbhour Solicitation)
///     - Packet Code 1     (Neigbhour Advertisement) 
///     - Packet Code 4     (DNS FQDN Request)
///     - Packet Code 5     (DNS FQDN Response)
///     - Packet Code 6     (DNS ILV Request)
///     - Packet Code 7     (DNS ILV Response)
///     - Packet Code 8     (Router Request)
///     - Packet Code 9     (Router Response)
async fn handle_jcmp_packet(emulator_socket: &EmulatorSocket, source_address: Ipv6Addr,  ilnp_header: INLPv6Packet, jcmp_payload: &[u8])
{

    // check for neighbour solicitation
    if jcmp_payload[0] == 0 {

        // check the solicitation is for us and we didn't send out the request
        if CONFIG.node.nid  == ilnp_header.destination_identifier() && CONFIG.node.nid != ilnp_header.source_identifier() {

            // count jcmp request
            match PCB.lock() {
                Ok(mut pcb) => {
                    pcb.nd_solicitation_jcmp_rx += 1;
                },
                Err(_) => {}
            }

            // get the interface name for the locator we received
            match get_over_interface_by_locator(&ilnp_header.destination_locator()) {
                Ok(inf_name) => {

                    // respond with neighbour advertisement
                    match jcmp_tx_advertisement(emulator_socket, &ilnp_header.source_identifier(), &inf_name).await {
                        Ok(())  => {},
                        Err(err) => {
                            log_error(emulator_socket, &err).await;
                        }
                    }

                },
                Err(err) => {
                    log_error(emulator_socket, &err).await;
                }
            }

        }

    }

    // check for neighbour advertisement
    else if jcmp_payload[0] == 1 {

        // here we could check packet is meant for us but since ND is multicast we can collect all ads
        if CONFIG.node.nid != ilnp_header.source_identifier() {

            // count jcmp response
            match PCB.lock() {
                Ok(mut pcb) => {
                    pcb.nd_advertisement_jcmp_rx += 1;
                },
                Err(_) => {}
            }

            // parse the response
            match JCMP_ND_Advertisement::from_bytes(jcmp_payload) {
                Ok(jcmp_nd_pck) => {

                    // get the interface name for the locator we received
                    match get_over_interface_by_locator(&ilnp_header.source_locator()) {
                        Ok(interface_name) => {

                            // add source (intervace, IPv6, port) mapped to source NID
                            match NID_ADDRESS_RESOLUTION_TABLE.lock() {
                                Ok(mut map)  => {
                                    map.insert(ilnp_header.source_identifier(), (interface_name, source_address, jcmp_nd_pck.destination_port), Duration::from_secs(CONFIG.network.ND_TTL_S));
                                    return;
                                },
                                Err(_) => {}
                            }
                            log_error(emulator_socket, &format!("handle_jcmp_packet(): failed to insert ND AD to NID_INTERFACE_IP_TABLE")).await;

                        },
                        Err(err)  => {
                            log_error(emulator_socket, &err).await;
                        }
                    }

                },
                Err(err) => {
                    log_error(emulator_socket, &err).await;
                }
            }

        }

    }

    // check for dns FQDN lookup
    else if jcmp_payload[0] == 4 {

        // check we didn't send out the request
        if CONFIG.node.nid != ilnp_header.source_identifier() {

            // count jcmp request
            match PCB.lock() {
                Ok(mut pcb) => {
                    pcb.dns_fqdn_query_jcmp_rx += 1;
                },
                Err(_) => {}
            }

            // parse the response
            match JCMP_DNS_FQDN_Query_Packet::from_bytes(jcmp_payload) {
                Ok(jcmp_query_pck) => {

                    match String::from_utf8(jcmp_query_pck.fqdn) {
                        Ok(fqdn) => {

                            // check dns name is ours
                            if emulator_socket.local_network.local_fqdn == fqdn {

                                // send dns response
                                match jcmp_tx_dns_fqdn_response(emulator_socket, &ilnp_header.source_identifier()).await {
                                    Ok(()) => {},
                                    Err(err) => {
                                        log_error(emulator_socket, &err).await;
                                    }
                                }

                            }
                        }, 
                        Err(err)  => {
                            log_error(emulator_socket, &format!("handle_ilnp_buffer(): couldn't convert fqdn from DNS query: {}", err)).await
                        }
                    }

                },
                Err(err) => {
                    log_error(emulator_socket, &err).await;
                }

            }

        }

    }

    // check for DNS FQDN response
    else if jcmp_payload[0] == 5 {

        // check we didn't send the response
        // collect all responses except ours to reduce packet overhead
        if CONFIG.node.nid != ilnp_header.source_identifier() {

            // count jcmp response
            match PCB.lock() {
                Ok(mut pcb) => {
                    pcb.dns_fqdn_response_jcmp_rx += 1;
                },
                Err(_) => {}
            }

            // parse the response
            match JCMP_DNS_FQDN_Response_Packet::from_bytes(jcmp_payload) {
                Ok(jcmp_response_pck) => {

                    match String::from_utf8(jcmp_response_pck.fqdn) {
                        Ok(fqdn) => {

                            // insert the response into the name resolution table
                            match insert_into_name_ilv_table((fqdn, ilnp_header.source_identifier(), ilnp_header.source_locator()), jcmp_response_pck.ttl as u64) {
                                Ok(()) => {},
                                Err(err) => {
                                    log_error(emulator_socket, &err).await;
                                }
                            }

                        },
                        Err(err) => {
                            log_error(emulator_socket, &format!("handle_jcmp_packet(): failed convert fqdn to string: {}", err)).await;
                        }
                    }

                },
                Err(err)  => {
                    log_error(emulator_socket, &err).await;
                }
            }

        }

    }

    // check for DNS ILV request
    else if jcmp_payload[0] == 6 {

        // check ILV query is for us and we didn't send it
        if CONFIG.node.nid  == ilnp_header.destination_identifier() && CONFIG.node.nid != ilnp_header.source_identifier() {

            // count jcmp request
            match PCB.lock() {
                Ok(mut pcb) => {
                    pcb.dns_ilv_query_jcmp_rx += 1;
                },
                Err(_) => {}
            }
            
            // send a DNS ILV response
            match jcmp_tx_dns_ilv_response(emulator_socket, &ilnp_header.source_identifier()).await {
                Ok(()) => {},
                Err(err) => {
                    log_error(emulator_socket, &err).await;
                }
            }

        }

    }

    // check for DNS ILV response
    else if jcmp_payload[0] == 7 {

        // check it was not sent by us
        // receive all requests expect ours to reduce packet overhead
        if CONFIG.node.nid != ilnp_header.source_identifier() {

            // count jcmp response
            match PCB.lock() {
                Ok(mut pcb) => {
                    pcb.dns_ilv_response_jcmp_rx += 1;
                },
                Err(_) => {}
            }

            // parse the response
            let jcmp_ilvresponse_payload:[u8; 2] = match jcmp_payload.try_into() {
                Ok(jcmp_ilvresponse_payload) => {
                    jcmp_ilvresponse_payload
                },
                Err(err) => {
                    log_error(emulator_socket, &format!("handle_jcmp_packet(): failed to serialise jcmp dns ilv response: {}", err)).await;
                    return;
                }
            };

            // add the results in the name resolution table
            let jcmp_ilvresponse_pck= JCMP_DNS_ILV_Response_Packet::from_bytes(jcmp_ilvresponse_payload);
            match insert_into_nid_ilv_table((ilnp_header.source_identifier(), ilnp_header.source_locator()), jcmp_ilvresponse_pck.ttl() as u64) {
                Ok(()) => {},
                Err(err) => {
                    log_error(emulator_socket, &err).await;
                }
            }

        }

    }

    // check for router request
    else if jcmp_payload[0] == 8 {

        // check it wasn't sent by us
        if CONFIG.node.nid != ilnp_header.source_identifier() {

            // only routers can forward so only routers should respond to this
            if CONFIG.node.router {

                // count jcmp request
                match PCB.lock() {
                    Ok(mut pcb) => {
                        pcb.router_request_jcmp_rx += 1;
                    },
                    Err(_) => {}
                }

                // parse the request
                let jcmp_routerrequest_payload: [u8; 10] = match jcmp_payload.try_into() {
                    Ok(jcmp_routerrequest_payload) => {
                        jcmp_routerrequest_payload
                    },
                    Err(err) => {
                        log_error(emulator_socket, &format!("handle_jcmp_packet(): failed to serialise jcmp router request: {}", err)).await;
                        return;
                    }
                };
                let jcmp_routerrequest_pck = JCMP_Router_Request::from_bytes(jcmp_routerrequest_payload);
                
                // if max hop count reached stop the request
                // this avoids infinite looping
                let current_hop_count = jcmp_routerrequest_pck.hop_count();
                if current_hop_count > CONFIG.network.AD_MAX_HOPS {
                    return;
                }

                // get interface name for the source of the jcmp request
                let lookup_locator = jcmp_routerrequest_pck.destination_locator();
                match get_over_interface_by_locator(&ilnp_header.source_locator()) {
                    Ok(source_interface_name) => {

                        // first check if we're connected to target locator
                        // if yes respond with a request hop count set to 1
                        match get_over_interface_by_locator(&lookup_locator) {
                            Ok(_) => {
                                let _ = jcmp_tx_router_response(emulator_socket, &lookup_locator, &ilnp_header.source_identifier(), &source_interface_name, &1).await;   
                            },
                            Err(_) =>  {

                                // if no we need to discover the path to the target locator
                                // 1 is added to the request hop count to stop infinite looping
                                // 1 is added to the response to count the hops back to the source
                                match handle_path_discovery(&emulator_socket, Some(&source_interface_name), &lookup_locator, &(current_hop_count+1)).await {
                                    Ok((_, _, _, hop_count)) => {
                                        let _ = jcmp_tx_router_response(&emulator_socket, &lookup_locator, &ilnp_header.source_identifier(), &source_interface_name, &(hop_count+1)).await;
                                    },
                                    Err(err) => {
                                        log_error(&emulator_socket, &err).await;
                                    }
                                }

                            }
                        }

                    },
                    Err(err) => {
                        log_error(emulator_socket, &err).await;
                    }
                }

            }

        }

    }

    // check for router response
    else if jcmp_payload[0] == 9 {

        // check we didn't send the rsponse
        if CONFIG.node.nid != ilnp_header.source_identifier() {

            // count jcmp response
            match PCB.lock() {
                Ok(mut pcb) => {
                    pcb.router_response_jcmp_rx += 1;
                },
                Err(_) => {}
            }

            // parse the response
            let jcmp_routerresponse_payload: [u8; 11] = match jcmp_payload.try_into() {
                Ok(jcmp_routerresponse_payload) => {
                    jcmp_routerresponse_payload
                },
                Err(err) => {
                    log_error(emulator_socket, &format!("handle_jcmp_packet(): failed to serialise jcmp router response: {}", err)).await;
                    return;
                }
            };
            let jcmp_routerresponse_pck = JCMP_Router_Response::from_bytes(jcmp_routerresponse_payload);
            
            // retrieve metrics
            let lookup_locator = jcmp_routerresponse_pck.destination_locator();
            let hop_count = jcmp_routerresponse_pck.hop_count();

            // get interface name for the network we receive the response in
            match get_over_interface_by_locator(&ilnp_header.source_locator()) {
                Ok(interface_name) => {

                    // check if entry already exists
                    match lookup_forwarding_table(&ilnp_header.source_identifier(), &lookup_locator) {
                        Ok(entry) => {

                            // if new entry has a better hop count replace it
                            if entry.3 > hop_count {
                                match insert_into_forwarding_table((ilnp_header.source_identifier(), lookup_locator, interface_name, hop_count), jcmp_routerresponse_pck.ttl() as u64) {
                                    Ok(()) => {},
                                    Err(err) => {
                                        log_error(emulator_socket, &err).await;
                                    }
                                }
                            }
                        },
                        Err(_) => {

                            // insert new entry in the forwarding table
                            match insert_into_forwarding_table((ilnp_header.source_identifier(), lookup_locator, interface_name, hop_count), jcmp_routerresponse_pck.ttl() as u64) {
                                Ok(()) => {},
                                Err(err) => {
                                    log_error(emulator_socket, &err).await;
                                }
                            }
                        }
                    }

                },
                Err(err) => {
                    log_error(emulator_socket, &err).await;
                }
            }

        }

    }

    else {
        log_error(emulator_socket, &format!("handle_jtp_packet(): jcmp packet code {:?} not supported", jcmp_payload[0])).await;
    }


}


/// Handles forwarding a packet
pub async fn handle_router_forward(emulator_socket: &EmulatorSocket, ilnp_pck: &INLPv6Packet, payload: &[u8])
    -> Result<(), String>
{

    // get interface name for locator received
    match get_over_interface_by_locator(&ilnp_pck.destination_locator()) {

        // connected to network packet is supposed to be forwarded to
        Ok(interface_name) => {

            let destination_nid = ilnp_pck.destination_identifier();
            let (destination_address, destination_port) = handle_destination_nid(emulator_socket, &destination_nid, &interface_name).await?;

            // create the ILNPv6 header
            let mut pck_vec: Vec<u8> = ilnp_pck.into_bytes().to_vec();
            pck_vec.extend_from_slice(&payload);

            // forward packet to node
            let _ = underlay_uni_tx(emulator_socket, &destination_address, &destination_port, &pck_vec).await?;
            Ok(())

        },

        // need to forward packet to another router
        Err(_) => {
            
            // path discovery
            match handle_path_discovery(emulator_socket, None, &ilnp_pck.destination_locator(), &0).await {
                Ok((router_nid, _, interface_name, _)) => {

                    // address resolution
                    match handle_destination_nid(emulator_socket, &router_nid, &interface_name).await {
                        Ok((ipv6, port)) => {

                            // create the ILNPv6 header
                            let mut pck_vec: Vec<u8> = ilnp_pck.into_bytes().to_vec();
                            pck_vec.extend_from_slice(&payload);
                            
                            // forward packet to router
                            let _ = underlay_uni_tx(emulator_socket, &ipv6, &port, &pck_vec).await?;
                            Ok(())

                        },
                        Err(err) => {
                            Err(err)
                        }
                    }

                },
                Err(err) => {
                    Err(err)
                }
            }

        }
    }

}

/// Address Resolution function
pub async fn handle_destination_nid(emulator_socket: &EmulatorSocket, destination_nid:&u64, interface_name: &String)
    -> Result<(Ipv6Addr, u16), String>
{

    // IPv6 and port
    let mut ipv6_address = Ipv6Addr::UNSPECIFIED;
    let mut destination_port: u16 = 0;
    
    // attempt ND_RETRANSMIT_LIMIT times
    let mut attempt = 0;
    while attempt < CONFIG.network.ND_RETRANSMIT_LIMIT && ipv6_address == Ipv6Addr::UNSPECIFIED && destination_port == 0 {

        // lookup in the table first
        match NID_ADDRESS_RESOLUTION_TABLE.lock() {
            Ok(map) => {
                if let Some((_, des_addr, dest_port)) = map.get(destination_nid) {
                    ipv6_address = des_addr.clone();
                    destination_port = dest_port.clone();
                }
            },
            Err(err) => {
                return Err(format!("handle_destination_nid(): failed to lock NID_INTERFACE_IP_TABLE: {}", err));
            }
        }

        // if nothing in the table send solicitation
        if ipv6_address == Ipv6Addr::UNSPECIFIED && destination_port == 0 {
            let _ = jcmp_tx_solicitation(emulator_socket, destination_nid, interface_name).await;
            attempt += 1;

            // timeout
            tokio::time::sleep(Duration::from_millis(CONFIG.network.ND_RTO_MS)).await;
        }
    }

    // if fails then host is unreachable
    if ipv6_address != Ipv6Addr::UNSPECIFIED && destination_port != 0 {
        Ok((ipv6_address, destination_port))
    } else {
        Err(format!("handle_destination_nid(): host {} unreachable", destination_nid))
    }

}


/// FQDN Name Resolution function
pub async fn handle_destination_fqdn(emulator_socket: &EmulatorSocket, destination_fqdn:&String)
    -> Result<Vec<(u64, u64)>, String>
{

    // attempt ND_RETRANSMIT_LIMIT times
    let mut attempt = 0;
    while attempt < CONFIG.network.ND_RETRANSMIT_LIMIT {

        // check the name resolution table first
        match lookup_name_ilv_table(destination_fqdn) {
            Ok(entries) => {

                if entries.len() != 0 {
                    let result: Vec<(u64, u64)> = entries.into_iter()
                        .map(|(_, nid, loc)| (nid, loc))
                        .collect();
                    return Ok(result);
                }
                else {

                    // send DNS query if nothing is found
                    let _ = jcmp_tx_dns_fqdn_query(emulator_socket, destination_fqdn).await;
                    attempt += 1;
                }

            },
            Err(err) => {
                return Err(err);
            }
        }

        // timeout
        thread::sleep(Duration::from_millis(CONFIG.network.ND_RTO_MS));

    }

    Err(format!("handle_destination_name(): could not establish {}'s locator and identifier", destination_fqdn))

}

/// ILV Name Resolution function
pub async fn handle_destination_ilv(emulator_socket: &EmulatorSocket, destination_nid: &u64)
    -> Result<Vec<(u64, u64)>, String>
{

    // attempt ND_RETRANSMIT_LIMIT times
    let mut attempt = 0;
    while attempt < CONFIG.network.ND_RETRANSMIT_LIMIT {

        // check the table first
        match lookup_nid_ilv_table(&destination_nid) {
            Ok(entries) => {
                if entries.len() != 0 { 
                    return Ok(entries);
                }
                else {

                    // if nothing found send a DNS query
                    let _ = jcmp_tx_dns_ilv_query(emulator_socket, destination_nid).await;
                    attempt += 1;
                }
            },
            Err(err)  => {
                return Err(err);
            }
        }

        // timeout
        thread::sleep(Duration::from_millis(CONFIG.network.ND_RTO_MS));
        
    }

    Err(format!("handle_destination_ILV(): could not establish {}'s locator and identifier", destination_nid))

}


/// Path Discovery function
pub async fn handle_path_discovery(emulator_socket: &EmulatorSocket, source_interface: Option<&String> , lookup_locator: &u64, current_hop_count: &u8)
    -> Result<(u64, u64, String, u8), String>
{

    let mut disc_done = false;
    let start_time = Instant::now();
    let loop_duration = Duration::from_millis(CONFIG.network.AD_HOC_TIMEOUT_MS);

    // loop until timeout
    while start_time.elapsed() < loop_duration {

        // check forwarding table
        match lookup_forwarding_table_route(lookup_locator) {
            Ok(entry) => {

                return Ok(entry);

            },
            Err(_) => {

                // if not found and router request not sent then send it
                if !disc_done {
                
                    // get all interfaces
                    match get_over_interfaces() {
                        Ok(interfaces) => {

                            // send out discovery to all networks
                            for (interface_name, _) in interfaces {
                                if let Some(si) = source_interface {
                                    if &interface_name == si { continue; }
                                }
                                let _ = jcmp_tx_router_request(emulator_socket, lookup_locator, &interface_name, current_hop_count).await;
                            }
                
                            disc_done = true;

                        },
                        Err(err) => {
                            return Err(err);
                        }
                    }
        
                    
                }

            }
        }

        // timeout
        tokio::time::sleep(Duration::from_nanos(CONFIG.network.AD_HOC_RTO_NS)).await;
    }

    Err(format!("handle_path_discovery(): failed to resolve route for 0x{:016X} skipping {:?}\n", lookup_locator, source_interface))

}