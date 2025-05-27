#![allow(warnings)]

use modular_bitfield_msb::{bitfield, prelude::{B4, B8, B16, B20, B64}};

/*******************************************/
/// ILNP Packets
/// This is the link layer packets of our overlay network
#[bitfield]
#[derive(Debug, Clone, Copy)]
pub struct INLPv6Packet {
    pub version: B4,
    pub traffic_class: B8,
    pub flow_label: B20,
    pub payload_length: B16,
    pub next_header: B8,
    pub hop_limit: B8,
    pub source_locator: B64,
    pub source_identifier: B64,
    pub destination_locator: B64,
    pub destination_identifier: B64
}
/*******************************************/



/*******************************************/
/// JCMP Packets
/// This is the control plane packet built on top of ILNP
pub trait JCMP_Pck: Send + Sync {
    fn into_bytes(&self) -> Vec<u8>;
    fn get_packet_code(&self) -> u8;
}

/// JCMP Basic Packet
/// This is the JCMP header, packet code is required in the JCMP protocol
///     - ND Solicitation (0x00)
///     - ND Advertisement (0x01)
///     - ND Router Solicitation (0x02)
///     - ND Router Advertisement (0x03)
///     - DNS IlV Query (0x06)
#[bitfield]
#[derive(Debug, Clone, Copy)]
pub struct JCMP_Basic_Pck {
    pub packet_code: B8
}
impl JCMP_Pck for JCMP_Basic_Pck {
    fn into_bytes(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }
    fn get_packet_code(&self) -> u8 {
        self.packet_code()
    }
}

#[derive(Debug)]
pub struct JCMP_ND_Advertisement {
    pub header: JCMP_Basic_Pck,
    pub destination_port: u16
}
impl JCMP_ND_Advertisement {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let header_size = std::mem::size_of::<JCMP_Basic_Pck>();
        if bytes.len() < header_size + 2 {
            return Err("JCMP_ND_Neighbour_Advertisement::from_bytes(): header too small".to_string());
        }
        let header_array: [u8; 1] = match bytes[..header_size].try_into() {
            Ok(header_array) => {
                header_array
            },
            Err(err) => {
                return Err(format!("JCMP_ND_Neighbour_Advertisement::from_bytes(): header converting issue: {}", err));
            }
        };

        let header = JCMP_Basic_Pck::from_bytes(header_array);
        let destination_port = u16::from_be_bytes(bytes[header_size..header_size + 2].try_into().unwrap());
        Ok(JCMP_ND_Advertisement {
            header,
            destination_port,
        })
    }
}
impl JCMP_Pck for JCMP_ND_Advertisement {
    fn into_bytes(&self) -> Vec<u8> {
        let header_bytes = self.header.into_bytes();
        let destination_port_bytes = self.destination_port.to_be_bytes();
        let mut bytes = Vec::with_capacity(header_bytes.len() + 2);
        bytes.extend_from_slice(&header_bytes);
        bytes.extend_from_slice(&destination_port_bytes);
        bytes
    }
    fn get_packet_code(&self) -> u8 {
        self.header.packet_code()
    }
}

/// JCMP DNS FQDN Query Packet
/// This packet is used to query the fake DNS by FQDN
///     - DNS FQDN Query (0x04)
#[derive(Debug)]
pub struct JCMP_DNS_FQDN_Query_Packet {
    pub header: JCMP_Basic_Pck,
    pub fqdn: Vec<u8>
}
impl JCMP_DNS_FQDN_Query_Packet {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {   
        let header_size = std::mem::size_of::<JCMP_Basic_Pck>();
        let header_bytes = &bytes[..header_size];
        match header_bytes.try_into() as Result<[u8; 1], _> {
            Ok(header_array) => {
                let header = JCMP_Basic_Pck::from_bytes(header_array);
                let fqdn = bytes[header_size..].to_vec();
                Ok(JCMP_DNS_FQDN_Query_Packet { header, fqdn })
            },
            Err(err) => {
                Err(format!("JCMPDNSQueryPacket::from_bytes(): {}", err))
            }
        }
    }
}
impl JCMP_Pck for JCMP_DNS_FQDN_Query_Packet {
    fn into_bytes(&self) -> Vec<u8> {
        let header_bytes = self.header.into_bytes();
        let mut bytes = Vec::with_capacity(header_bytes.len() + self.fqdn.len());
        bytes.extend_from_slice(&header_bytes);
        bytes.extend_from_slice(&self.fqdn);
        bytes
    }
    fn get_packet_code(&self) -> u8 {
        self.header.packet_code()
    }
}

/// JCMP DNS FQDN Response Packet
/// This packet is used to respond to the fake DNS by FQDN
///     - DNS FQDN Response (0x05)
#[derive(Debug)]
pub struct JCMP_DNS_FQDN_Response_Packet {
    pub header: JCMP_Basic_Pck,
    pub ttl: u8,
    pub fqdn: Vec<u8>
}
impl JCMP_DNS_FQDN_Response_Packet {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {   
        let header_size = std::mem::size_of::<JCMP_Basic_Pck>();
        if bytes.len() < header_size + 1 {
            return Err("JCMPDNSResponsePacket::from_bytes(): header too small".to_string());
        }

        let header_array: [u8; 1] = match bytes[..header_size].try_into() {
            Ok(header_array) => {
                header_array
            },
            Err(err) => {
                return Err(format!("JCMPDNSResponsePacket: header converting issue: {}", err));
            }
        };

        let header = JCMP_Basic_Pck::from_bytes(header_array);
        let ttl = bytes[header_size];
        let fqdn = bytes[(header_size + 1)..].to_vec();
        Ok(JCMP_DNS_FQDN_Response_Packet { header, ttl, fqdn })

    }
}
impl JCMP_Pck for JCMP_DNS_FQDN_Response_Packet {
    fn into_bytes(&self) -> Vec<u8> {
        let header_bytes = self.header.into_bytes();
        let mut bytes = Vec::with_capacity(header_bytes.len() + 1 + self.fqdn.len());
        bytes.extend_from_slice(&header_bytes);
        bytes.push(self.ttl);
        bytes.extend_from_slice(&self.fqdn);
        bytes
    }
    fn get_packet_code(&self) -> u8 {
        self.header.packet_code()
    }
}


/// JCMP DNS ILV Response Packet
/// This packet is used to respond to the fake DNS by ILV
///     - DNS ILV Response (0x07)
#[bitfield]
#[derive(Debug, Clone, Copy)]
pub struct JCMP_DNS_ILV_Response_Packet {
    pub packet_code: B8,
    pub ttl: B8
}
impl JCMP_Pck for JCMP_DNS_ILV_Response_Packet {
    fn into_bytes(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }
    fn get_packet_code(&self) -> u8 {
        self.packet_code()
    }
}

/// JCMP RREQ Router Request Packet
/// This packet is used to request backwards learning across routers
///     - RREQ Request (0x08)
#[bitfield]
#[derive(Debug, Clone, Copy)]
pub struct JCMP_Router_Request {
    pub packet_code: B8,
    pub hop_count: B8,
    pub destination_locator: B64
}
impl JCMP_Pck for JCMP_Router_Request {
    fn into_bytes(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }
    fn get_packet_code(&self) -> u8 {
        self.packet_code()
    }
}

/// JCMP RRES Router Response Packet
/// This packet is used to respond to RREQ
///     - RREQ Response (0x09)
#[bitfield]
#[derive(Debug, Clone, Copy)]
pub struct JCMP_Router_Response {
    pub packet_code: B8,
    pub hop_count: B8,
    pub destination_locator: B64,
    pub ttl: B8
}
impl JCMP_Pck for JCMP_Router_Response {
    fn into_bytes(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }
    fn get_packet_code(&self) -> u8 {
        self.packet_code()
    }
}

/*******************************************/