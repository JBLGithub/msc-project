use crate::{layers::underlay_network::underlay_multi_tx, models::{network_models::EmulatorSocket, network_packets::{INLPv6Packet, JCMP_Basic_Pck, JCMP_DNS_FQDN_Query_Packet, JCMP_DNS_FQDN_Response_Packet, JCMP_DNS_ILV_Response_Packet, JCMP_ND_Advertisement, JCMP_Pck, JCMP_Router_Request, JCMP_Router_Response}}, services::network_services::{get_over_interface_by_name, get_over_interfaces}};

use super::{CONFIG, PCB};

/// NS - Neighbour Solicitation
pub async fn jcmp_tx_solicitation(emulator_socket: &EmulatorSocket, destination_nid:&u64, interface_name: &String)
    -> Result<(), String>
{

    // create nd solicitation
    let jcmp_pck = JCMP_Basic_Pck::new()
        .with_packet_code(0);

    let (source_locator, _) = get_over_interface_by_name(interface_name)?;

    // send the control message to all networks
    let _ = jcmp_tx(emulator_socket, destination_nid, &source_locator, interface_name, &jcmp_pck).await?;

    // count JCMP transmit
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.nd_solicitation_jcmp_tx += 1;
        },
        Err(_) => {}
    }

    Ok(())
}

/// NA - Neighbour Advertisement
pub async fn jcmp_tx_advertisement(emulator_socket: &EmulatorSocket, destination_nid: &u64, interface_name: &String)
    -> Result<(), String>
{
    // create nd advertisement
    let jcmp_pck = JCMP_ND_Advertisement {
        header: JCMP_Basic_Pck::new()
            .with_packet_code(1),
        destination_port: emulator_socket.local_network.local_port
    };

    // set destination location to our destination locator since it's ND
    let (source_locator, _) = get_over_interface_by_name(interface_name)?;
    let _ = jcmp_tx(emulator_socket, destination_nid, &source_locator, interface_name, &jcmp_pck).await?;

    // count JCMP transmit
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.nd_advertisement_jcmp_tx += 1;
        },
        Err(_) => {}
    }

    Ok(())
}

/// DNS - FQDN Query
pub async fn jcmp_tx_dns_fqdn_query(emulator_socket: &EmulatorSocket, destination_name: &String)
    -> Result<(), String>
{
    // placeholder
    let dns_holder:u64 = 0x0000000053535353;

    // create the packet
    let jcmp_dnsquery_pck = JCMP_DNS_FQDN_Query_Packet {
        header: JCMP_Basic_Pck::new()
            .with_packet_code(4),
        fqdn:  destination_name.clone().into_bytes()
    };

    // send the packet on dns interface
    let _ = jcmp_tx(emulator_socket, &dns_holder, &dns_holder, &"dns".to_string(), &jcmp_dnsquery_pck).await?;

    // count JCMP transmit
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.dns_fqdn_query_jcmp_tx += 1;
        },
        Err(_) => {}
    }

    Ok(())
}

/// DNS - FQDN Response
pub async fn jcmp_tx_dns_fqdn_response(emulator_socket: &EmulatorSocket, destination_nid: &u64)
    -> Result<(), String>
{
    // create the packet
    let jcmp_dnsresponse_pck = JCMP_DNS_FQDN_Response_Packet {
        header: JCMP_Basic_Pck::new().with_packet_code(5),
        ttl: CONFIG.network.DNS_TTL_S,
        fqdn: CONFIG.node.name.clone().into_bytes()
    };

    // get interfaces
    match get_over_interfaces() {
        Ok(interfaces) => {

            // send response for each locator we are connected to
            for (_, (source_locator, _)) in interfaces {
                let _ = jcmp_tx(emulator_socket, &destination_nid, &source_locator, &"dns".to_string(), &jcmp_dnsresponse_pck).await?;
                match PCB.lock() {
                    Ok(mut pcb) => {
                        pcb.dns_fqdn_response_jcmp_tx += 1;
                    },
                    Err(_) => {}
                }
            }

            Ok(())

        },
        Err(err) => {
            Err(err)
        }
    }

}

/// DNS - ILV Query
pub async fn jcmp_tx_dns_ilv_query(emulator_socket: &EmulatorSocket, destination_nid: &u64)
    -> Result<(), String>
{
    // placeholder
    let dns_holder:u64 = 0x0000000053535353;

    // create the packet
    let jcmp_ilvquery_pck = JCMP_Basic_Pck::new()
        .with_packet_code(6);

    // send request on DNS interface
    let _ = jcmp_tx(emulator_socket, destination_nid, &dns_holder, &"dns".to_string(), &jcmp_ilvquery_pck).await?;

    // count JCMP transmit
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.dns_ilv_query_jcmp_tx += 1;
        },
        Err(_) => {}
    }

    Ok(())
}

/// DNS - ILV Response
pub async fn jcmp_tx_dns_ilv_response(emulator_socket: &EmulatorSocket, destination_nid: &u64)
    -> Result<(), String>
{
    // create the packet
    let jcmp_ilvresponse_pck = JCMP_DNS_ILV_Response_Packet::new()
        .with_packet_code(7)
        .with_ttl(CONFIG.network.DNS_TTL_S);

    // get interfaces
    match get_over_interfaces() {
        Ok(interfaces) => {

            // send a response for each of the locators we are connected to
            for (_, (source_locator, _)) in interfaces {

                let _ = jcmp_tx(emulator_socket, destination_nid, &source_locator, &"dns".to_string(), &jcmp_ilvresponse_pck).await?;
                match PCB.lock() {
                    Ok(mut pcb) => {
                        pcb.dns_ilv_response_jcmp_tx += 1;
                    },
                    Err(_) => {}
                }

            }
            Ok(())

        },
        Err(err) => {
            Err(err)
        }
    }

}

/// JCMP - Router Request
pub async fn jcmp_tx_router_request(emulator_socket: &EmulatorSocket, lookup_locator: &u64, interface_name: &String, hop_count: &u8)
    -> Result<(), String>
{
    // placeholder
    let destination_nid:u64 = 0x00000000ff02ff02;

    // create the packet
    let jcmp_routerquery_pck = JCMP_Router_Request::new()
        .with_packet_code(8)
        .with_hop_count(hop_count.clone())
        .with_destination_locator(lookup_locator.clone());

    // send request
    let (source_locator, _) = get_over_interface_by_name(interface_name)?;
    let _ = jcmp_tx(emulator_socket, &destination_nid, &source_locator, interface_name, &jcmp_routerquery_pck).await?;

    // count JCMP transmit
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.router_request_jcmp_tx += 1;
        },
        Err(_) => {}
    }

    Ok(())
}

/// JCMP - Router Response
pub async fn jcmp_tx_router_response(emulator_socket: &EmulatorSocket, lookup_locator: &u64, destination_nid: &u64, interface_name: &String, hop_count: &u8)
    -> Result<(), String>
{
    // create the packet
    let jcmp_routerresponse_pck = JCMP_Router_Response::new()
        .with_packet_code(9)
        .with_hop_count(hop_count.clone())
        .with_destination_locator(lookup_locator.clone())
        .with_ttl(CONFIG.network.AD_HOC_TTL_S);

    // send request
    let (source_locator, _) = get_over_interface_by_name(interface_name)?;
    let _ = jcmp_tx(emulator_socket, destination_nid, &source_locator, interface_name, &jcmp_routerresponse_pck).await?;

    // count JCMP transmit
    match PCB.lock() {
        Ok(mut pcb) => {
            pcb.router_response_jcmp_tx += 1;
        },
        Err(_) => {}
    }

    Ok(())
}

// JCMP TX - Send Control Message
pub async fn jcmp_tx(emulator_socket: &EmulatorSocket, destination_nid: &u64, source_locator:&u64, interface_name: &String, jcmp_pck:&dyn JCMP_Pck)
    -> Result<(), String>
{
    let jcmp_buf = jcmp_pck.into_bytes();

    // get locator for given interface name
    let (destination_locator, _) = get_over_interface_by_name(interface_name)?;

    // create the ILNPv6 header
    let inlp_pck = INLPv6Packet::new()
        .with_version(6)
        .with_traffic_class(0)
        .with_flow_label(0)
        .with_payload_length(jcmp_buf.len() as u16)
        .with_next_header(150)
        .with_hop_limit(1)
        .with_source_locator(source_locator.clone())
        .with_source_identifier(emulator_socket.local_network.local_nid)
        .with_destination_locator(destination_locator.clone())
        .with_destination_identifier(destination_nid.clone())
        .into_bytes();

    let mut ilnp_pck_vec: Vec<u8> = inlp_pck.to_vec();
    ilnp_pck_vec.extend_from_slice(&jcmp_buf);

    // send the multicast packet
    let _ = underlay_multi_tx(emulator_socket, interface_name, &ilnp_pck_vec).await;

    Ok(())
}