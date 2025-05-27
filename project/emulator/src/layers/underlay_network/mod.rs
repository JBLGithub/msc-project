use std::collections::HashMap;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use under_socket::{create_multi_socket, create_unicast_socket};

pub mod under_socket;
use crate::layers::overlay_network::CONFIG;
use crate::services::network_services::get_over_interface_by_name;
use crate::services::{log_services::{log_error, log_info}, network_services::{get_under_interface_by_name, get_multicast_to_join}};
use crate::models::network_models::EmulatorSocket;

/// INTERFACES
///     - placeholder for different simulated networks the node is connected to
///     - maps interface name to (locator (L64), multicast Ipv6 address)
///     - e.g. use "multi1" instead of "ff02:0:0:5d73::1" for simplicity
pub static INTERFACES: Lazy<Mutex<HashMap<String, (u64, Ipv6Addr)>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Open Socket
///     - opening a socket at the underlayer
///     - create multicast socket to listen for JCMP packets
///     - create unicast socket to listen for JTP packets
///     - join the required multicast groups
pub async fn open_underlay_socket()
    -> Result<EmulatorSocket, String>
{

    // get interface information (uid, ipv6, interface index, nid, fqdn)
    let mut emulator_interface = get_under_interface_by_name(&"enp3s0".to_string())?;

    // DNS and Log multicast IPv6 address
    let dns_multi = Ipv6Addr::new(0xff02, 0, 0, emulator_interface.local_uid.clone(), 0, 0, 0x5353, 0x5353);
    let log_multi: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, emulator_interface.local_uid.clone(), 0, 0, emulator_interface.local_uid.clone(), emulator_interface.local_uid.clone());

    // create multicast socket
    match create_multi_socket(&emulator_interface, false) {
        Ok(mulcast_socket) => {

            // create unicast socket
            match  create_unicast_socket(&mut emulator_interface) {
                Ok(unicast_socket) => {

                    // create socket holder
                    let emulator_socket = EmulatorSocket {
                        mulcast_socket: Arc::new(mulcast_socket),
                        unicast_socket: Arc::new(unicast_socket),
                        local_network: emulator_interface.clone()
                    };

                    // log array while waiting to join the log multicast
                    let mut info_logs: Vec<String> = Vec::new();

                    // if node is not the node collecting log messages
                    // connect to required multicast groups
                    if !CONFIG.app.logger {

                        let mut interface_count = 0;

                        // get multicast groups to join from the config
                        let multi_ipv6s = get_multicast_to_join(CONFIG.node.networks.clone())?;

                        // for each group join it
                        for (locator, multi_ipv6) in multi_ipv6s {
                            match join_multicast(&emulator_socket, &multi_ipv6) {
                                Ok(()) => {

                                    // create interface name as placeholder for multifcast group
                                    let interface_name = format!("multi{}", interface_count);
                                    interface_count += 1;

                                    // insert new interface into the table
                                    match INTERFACES.lock() {
                                        Ok(mut map) => {
                                            map.insert(interface_name.clone(), (locator, multi_ipv6));
                                            info_logs.push(format!("open_underlay_socket(): succesfully joined: {:?}", multi_ipv6));
                                        },
                                        Err(err) => {
                                            return Err(format!("open_underlay_socket(): failed to lock INTERFACES: {}", err));
                                        }
                                    }

                                },
                                Err(err) => {
                                    return Err(format!("open_underlay_socket(): error joining network: {}:{}", multi_ipv6, err));
                                }
                            }
                        }

                        // join fake DNS multicast
                        match join_multicast(&emulator_socket, &dns_multi) {
                            Ok(()) => {

                                let dns_locator: u64 = 0x000053535353;
                                match INTERFACES.lock(){
                                    Ok(mut map) => {
                                        map.insert("dns".to_string(), (dns_locator, dns_multi));
                                        info_logs.push(format!("open_underlay_socket(): succesfully joined: {:?}", dns_multi));
                                    },
                                    Err(err) => {
                                        return Err(format!("open_underlay_socket(): failed to lock INTERFACES: {}", err));
                                    }
                                }

                            },
                            Err(err) => {
                                return Err(format!("open_underlay_socket(): error joining DNS network: {}:{}", dns_multi, err));
                            }
                        }

                    }

                    // join log multicast
                    match join_multicast(&emulator_socket, &log_multi)  {
                        Ok(()) => {

                            let log_locator: u64 = ((emulator_interface.local_nid.clone() as u64) << 16) | (emulator_interface.local_nid.clone() as u64);
                            match INTERFACES.lock(){
                                Ok(mut map) => {
                                    map.insert("log".to_string(), (log_locator, log_multi));
                                    info_logs.push(format!("open_underlay_socket(): succesfully joined: {:?}", log_multi));
                                },
                                Err(err) => {
                                    return Err(format!("open_underlay_socket(): failed to lock INTERFACES: {}", err));
                                }
                            }
                        },
                        Err(err)  => {
                            return Err(format!("open_underlay_socket(): error joining LOG network: {}:{}", log_multi, err));
                        }
                    }

                    // send logs to the log multicast group
                    for info_log in info_logs {
                        if CONFIG.app.logger {
                            println!("{}", info_log);
                        } else {
                            log_info(&emulator_socket, &info_log).await;
                        }
                    }

                    Ok(emulator_socket)

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

/// Join multicast group
///     - use the socket to join multicast group using "UdpSocket.join_multicast_v6()"
pub fn join_multicast(emulator_socket: &EmulatorSocket, multi_ipv6: &Ipv6Addr)
    -> Result<(), String>
{
    // join multicast group
    match emulator_socket.mulcast_socket.join_multicast_v6(multi_ipv6, emulator_socket.local_network.local_index) {
        Ok(()) => {
            Ok(())
        },
        Err(err) => { Err(format!("join_multicast() : {}", err)) }
    }
}

/// Close Socket
///     - leave all mutlicast groups
///     - leave DNS and Logs multicast
///     - drop the multicast and unicast socket
pub async fn close_underlay_socket(emulator_socket: EmulatorSocket)  
    -> Result<(), String>
{
    // leave multicast groups
    if !CONFIG.app.logger {

        // get number of groups to leave
        let multi_ipv6s = get_multicast_to_join(CONFIG.node.networks.clone())?;

        // leave multicast groups
        for (_, multi_ipv6) in multi_ipv6s {
            match leave_multicast(&emulator_socket, &multi_ipv6) {
                Ok(()) => {
                    log_info(&emulator_socket, &format!("close_underlay_socket(): successfully left network: {}", multi_ipv6)).await;
                },
                Err(err) => {
                    log_error(&emulator_socket, &format!("close_underlay_socket(): error leaving network: {}:{}", multi_ipv6, err)).await;
                }
            }
        }

        // leave DNS
        let dns_multi = Ipv6Addr::new(0xff02, 0, 0, emulator_socket.local_network.local_uid, 0, 0, 0x5353, 0x5353);
        match leave_multicast(&emulator_socket, &dns_multi) {
            Ok(()) => {
                log_info(&emulator_socket, &format!("close_underlay_socket(): successfully left network: {}", dns_multi)).await;
            },
            Err(err) => {
                log_error(&emulator_socket, &format!("close_underlay_socket(): error leaving network: {}:{}", dns_multi, err)).await;
            }
        }

    }

    // leave LOG
    let log_multi: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, emulator_socket.local_network.local_uid, 0, 0, emulator_socket.local_network.local_uid, emulator_socket.local_network.local_uid);
    match leave_multicast(&emulator_socket, &log_multi) {
        Ok(()) => {
            log_info(&emulator_socket, &format!("close_underlay_socket(): successfully left network: {}", log_multi)).await;
        },
        Err(err) => {
            return Err(format!("SYSTEMLOG: close_underlay_socket(): error leaving network: {}:{}", log_multi, err));
        }
    }

    // drop sockets
    drop(emulator_socket.mulcast_socket);
    drop(emulator_socket.unicast_socket);

    Ok(())
}

/// Leave multicast group
///     - use the socket to leave multicast group using "UdpSocket.leave_multicast_v6()"
pub fn leave_multicast(emulator_socket: &EmulatorSocket, multi_ipv6: &Ipv6Addr)
    -> Result<(), String>
{
    match emulator_socket.mulcast_socket.leave_multicast_v6(multi_ipv6, emulator_socket.local_network.local_index) {
        Ok(()) => {
            Ok(())
        },
        Err(err) => {
            Err(format!("leave_multicast() : {}", err))
        }
    }
}



/// TX Multicast UDP
///     - this function is to send packets in the multicast groups
///     - interface_name is the interface assigned to the group in INTERFACES for ease
///     - packet should include ILNP header at this point
pub async fn underlay_multi_tx(emulator_socket: &EmulatorSocket, interface_name: &String, pck: &[u8])
    -> Result<(), String>
{

    // get interface multicast IPv6 address
    let interface = get_over_interface_by_name(interface_name)?;

    // send packet over multicast
    let dest_addr = SocketAddrV6::new(interface.1, emulator_socket.local_network.local_uid, 0, emulator_socket.local_network.local_index);
    match emulator_socket.mulcast_socket.send_to(&pck, &dest_addr).await {
        Ok(_) => {
            Ok(())
        },
        Err(err) => {
            Err(format!("underlay_multi_tx(): error sending packet to: {}:{}", interface.1.clone(), err))
        }
    }

}


/// TX Unicast UDP
///     - this function is to send packets through unicast
///     - requires a destination local IPv6 (0xfe80) and an ephemeral port of the destination node
///     - packet should include ILNP header at this point
pub async fn underlay_uni_tx(emulator_socket: &EmulatorSocket, destination_address: &Ipv6Addr, destination_port: &u16, pck: &[u8])
    -> Result<(), String>
{

    // send packet over unicast
    let dest_addr = SocketAddrV6::new(destination_address.clone(), destination_port.clone(), 0, emulator_socket.local_network.local_index);
    match emulator_socket.unicast_socket.send_to(&pck, &dest_addr).await {
        Ok(_) => {
            Ok(())
        },
        Err(err) => {
            Err(format!("underlay_uni_tx(): error sending packet to: {}:{}", destination_address, err))
        }
    }

}