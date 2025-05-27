use pnet::datalink::{self};
use pnet::ipnetwork::IpNetwork;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::net::Ipv6Addr;
use std::time::Duration;
use std::hash::{Hash, Hasher};

use crate::layers::overlay_network::{CONFIG, NAME_ILV_TABLE, NID_ILV_TABLE};
use crate::layers::underlay_network::INTERFACES;
use crate::layers::overlay_network::LOCATOR_FORWARDING_TABLE;

use crate::models::network_models::EmulatorLocalNetwork;
use crate::services::config_services::get_uid;

/// Create network configurations based on the number of networks needed.
pub fn get_multicast_to_join(networks: Vec<u16>)
    -> Result<HashMap<u64, Ipv6Addr>, String>
{
    // get the user's uid
    let uid = get_uid()?;

    // create the overlay networks
    let mut mutli_ipv6s: HashMap<u64, Ipv6Addr> = HashMap::new();
    for n in networks {
        mutli_ipv6s.insert(n as u64, Ipv6Addr::new(0xff02, 0, 0, uid, 0, 0, 0, n));
    }

    Ok(mutli_ipv6s)
}


pub fn get_under_interface_by_name(interface_name: &String)
    -> Result<EmulatorLocalNetwork, String>
{

    let uid = get_uid()?;

    let interfaces = datalink::interfaces();
    for interface in interfaces {
        if &interface.name != interface_name { continue; }

        let index = interface.index;
        for ip_info in interface.ips {

            match ip_info {
                IpNetwork::V6(addr) => {
                    let new_interface = EmulatorLocalNetwork {
                        local_uid: uid,
                        local_index: index,
                        local_nid: CONFIG.node.nid,
                        local_fqdn: CONFIG.node.name.clone(),
                        local_ipv6: addr.ip(),
                        local_port: 0
                    };
                    return Ok(new_interface);
                },
                _ => { continue; }
            }

        }

    }
    
    Err(format!("get_interface(): couldn't find interface {}", interface_name))

}


/// INTERFACES Action
/// ******************************************************
pub fn get_over_interface_by_name(interface_name: &String)
    -> Result<(u64, Ipv6Addr), String>
{
    match INTERFACES.lock() {
        Ok(interfaces) => {
            if let Some(result) =  interfaces.get(interface_name)  {
                Ok(result.clone())
            } else  {
                Err(format!("get_interface_by_name(): interface not found"))
            }
        },
        Err(err) => {
            Err(format!("get_interface_by_name(): failed to lock INTERFACES: {}", err))
        }
    }
}

pub fn get_over_interface_by_locator(locator: &u64)
    -> Result<String, String>
{
    match INTERFACES.lock() {
        Ok(interfaces) => {
            for (inf_name, (current_locator, _)) in interfaces.iter() {
                if current_locator == locator {
                    return Ok(inf_name.clone());
                }
            }
            Err(format!("get_interface_for_locator(): interface not found"))
        },
        Err(err) => {
            Err(format!("get_interface_for_locator(): failed to lock INTERFACES: {}", err))
        }
    }
}

pub fn get_over_interfaces() 
-> Result<Vec<(String, (u64, Ipv6Addr))>, String> 
{
    match INTERFACES.lock() {
        Ok(interfaces) => {
            let result: Vec<(String, (u64, Ipv6Addr))> = interfaces
                .iter()
                .filter(|(key, _)| key != &"dns" && key != &"log")
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            Ok(result)
        },
        Err(err) => {
            Err(format!("get_over_interfaces(): failed to lock INTERFACES: {}", err))
        }
    }
}

pub fn get_over_locators()
    -> Result<Vec<u64>, String> 
{
    match INTERFACES.lock() {
        Ok(interfaces) => {
            let result: Vec<u64> = interfaces
                .iter()
                .filter(|(key, _)| key != &"log")
                .map(|(_, value)| value.0)
                .collect();
            Ok(result)
        },
        Err(err) => {
            Err(format!("get_over_interfaces(): failed to lock INTERFACES: {}", err))
        }
    }
}
// ******************************************************


/// DNS TABLES Action
/// ******************************************************
pub fn insert_into_name_ilv_table(entry: (String, u64, u64), ttl:u64) 
    -> Result<(), String>
{
    // generate key
    let mut hasher = DefaultHasher::new();
    let key = (entry.1, entry.2);
    key.hash(&mut hasher);
    let hash = hasher.finish();

    // insert into forwarding table
    match NAME_ILV_TABLE.lock() {
        Ok(mut map) => {
            map.insert(hash, entry, Duration::from_secs(ttl));
            Ok(())
        },
        Err(err) => {
            Err(format!("insert_into_name_ilv_table(): failed to lock NAME_ILV_TABLE: {}", err))
        }
    }
}
pub fn lookup_name_ilv_table(destination_fqdn: &String)
    -> Result<Vec<(String, u64, u64)>, String>
{
    match NAME_ILV_TABLE.lock() {
        Ok(map) => {
            let mut result: Vec<(String, u64, u64)> = Vec::new();
            for (_, (fqdn, nid, loc)) in map.clone().iter() {
                if fqdn == destination_fqdn {
                    result.push((fqdn.clone(), nid.clone(), loc.clone()));
                }
            }
            Ok(result)
        },
        Err(err) => {
            Err(format!("lookup_name_ilv_table(): failed to lock NAME_ILV_TABLE: {}", err))
        }
    }
}
pub fn insert_into_nid_ilv_table(entry: (u64, u64), ttl:u64) 
    -> Result<(), String>
{
    // generate key
    let mut hasher = DefaultHasher::new();
    entry.hash(&mut hasher);
    let hash = hasher.finish();

    // insert into forwarding table
    match NID_ILV_TABLE.lock() {
        Ok(mut map) => {
            map.insert(hash, entry, Duration::from_secs(ttl));
            Ok(())
        },
        Err(err) => {
            Err(format!("insert_into_nid_ilv_table(): failed to lock NID_ILV_TABLE: {}", err))
        }
    }
}
pub fn lookup_nid_ilv_table(destination_nid: &u64)
    -> Result<Vec<(u64, u64)>, String>
{
    match NID_ILV_TABLE.lock() {
        Ok(map) => {
            let mut result: Vec<(u64, u64)> = Vec::new();
            for (_, (nid, loc)) in map.clone().iter() {
                if nid == destination_nid {
                    result.push((nid.clone(), loc.clone()));
                }
            }
            Ok(result)
        },
        Err(err) => {
            Err(format!("lookup_nid_ilv_table(): failed to lock NID_ILV_TABLE: {}", err))
        }
    }
}
// ******************************************************


/// ROUTING TABLES Action
/// ******************************************************
pub fn insert_into_forwarding_table(entry: (u64, u64, String, u8), ttl:u64) 
    -> Result<(), String>
{
    // generate key
    let mut hasher = DefaultHasher::new();
    let key = (entry.0, entry.1);
    key.hash(&mut hasher);
    let hash = hasher.finish();

    // insert into forwarding table
    match LOCATOR_FORWARDING_TABLE.lock() {
        Ok(mut map) => {
            map.insert(hash, entry, Duration::from_secs(ttl));
            Ok(())
        },
        Err(err) => {
            Err(format!("insert_into_forwarding_table(): failed to lock LOCATOR_FORWARDING_TABLE: {}", err))
        }
    }
}
pub fn lookup_forwarding_table(identifier: &u64, locator: &u64)
    -> Result<(u64, u64, String, u8), String>
{
    match LOCATOR_FORWARDING_TABLE.lock() {
        Ok(map) => {
            for (_, entry) in map.clone().iter() {
                if &entry.0 == identifier && &entry.1 == locator {
                    return Ok(entry.clone());
                }
            }
            Err(format!("lookup_forwarding_table(): failed to entry in forwarding table"))
        },
        Err(err) => {
            Err(format!("lookup_forwarding_table(): failed to lock LOCATOR_FORWARDING_TABLE: {}", err))
        }
    }
}
pub fn lookup_forwarding_table_route(locator: &u64)
    -> Result<(u64, u64, String, u8), String>
{
    let mut result: (u64, u64, String, u8) = (0, 0, "".to_string(), 0);
    match LOCATOR_FORWARDING_TABLE.lock() {
        Ok(map) => {
            for (_, entry) in map.clone().iter() {
                if &entry.1 == locator && result.1 == 0 {
                    result = entry.clone();
                }
                else if &entry.1 == locator && entry.3 < result.3 {
                    result = entry.clone();
                }
            }
            if result.0 != 0 && result.1 != 0 {
                Ok(result)
            } else {
                Err(format!("lookup_forwarding_table(): failed to entry in forwarding table"))
            }
        },
        Err(err) => {
            Err(format!("lookup_forwarding_table(): failed to lock LOCATOR_FORWARDING_TABLE: {}", err))
        }
    }
}
// ******************************************************

/* 
pub fn print_forwarding_table() {
    match LOCATOR_FORWARDING_TABLE.lock() {
        Ok(map) => {
            println!();
            for (key, value) in map.clone().iter() {
                println!("{:?} => (0x{:016X}, 0x{:016X}, {:?}, {:?})", key, value.0, value.1, value.2, value.3);
            }
        },
        Err(_) => {}
    }
} */