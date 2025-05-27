use std::net::{Ipv6Addr, SocketAddrV6, UdpSocket as StdUdpSocket};
use libc::{fcntl, F_GETFL, F_SETFL, O_NONBLOCK};
use tokio::net::UdpSocket as TokioUdpsocket;
use std::os::unix::io::FromRawFd;

use crate::{layers::overlay_network::CONFIG, models::network_models::EmulatorLocalNetwork};

/// Create multicast socket
///     - can choose if its blocking or not
///     - setup socket config using libc
///     - return TokioUDPSocket
pub fn create_multi_socket(emulator_interface: &EmulatorLocalNetwork, blocking: bool)
    -> Result<TokioUdpsocket, String>
{

    // multi socket address
    // if logger just listen on the logger multicast
    let socket_addr = if CONFIG.app.logger {
        let log_multi: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, emulator_interface.local_uid.clone(), 0, 0, emulator_interface.local_uid.clone(), emulator_interface.local_uid.clone());
        SocketAddrV6::new(log_multi, emulator_interface.local_uid, 0, emulator_interface.local_index)
    } 
    
    // else listen to all multicast groups we are connected to
    else {
        SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, emulator_interface.local_uid, 0, emulator_interface.local_index)
    };

    // create empty IPv6 UDP socket using libc
    let fd = unsafe { libc::socket(libc::AF_INET6, libc::SOCK_DGRAM, 0) };
    if fd < 0 {
        return Err(format!("create_multi_socket(): failed to create socket: {}", std::io::Error::last_os_error()));
    }
    
    // source: https://stackoverflow.com/questions/40468685/how-to-set-the-socket-option-so-reuseport-in-rust
    // set SO_REUSEPORT
    unsafe {
        let reuse_port: libc::c_int = 1;
        if libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_REUSEPORT,
            &reuse_port as *const _ as *const libc::c_void,
            std::mem::size_of_val(&reuse_port) as libc::socklen_t
        ) != 0
        {
            return Err(format!("create_multi_socket(): failed to setup SO_REUSEPORT: {}", std::io::Error::last_os_error()));
        }
    }

    // set SO_REUSEADDR
    unsafe {
        let reuse_addr: libc::c_int = 1;
        if libc::setsockopt(
            fd, 
            libc::SOL_SOCKET, 
            libc::SO_REUSEADDR, 
            &reuse_addr as *const _ as *const libc::c_void, 
            std::mem::size_of_val(&reuse_addr) as libc::socklen_t
        ) != 0 {
            return Err(format!("create_multi_socket(): failed to setup SO_REUSEADDR: {}", std::io::Error::last_os_error()));
        }
    }

    // set IPV6_MULTICAST_IF
    unsafe {
        if libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_IF,
            &emulator_interface.local_index as *const _ as *const libc::c_void,
            std::mem::size_of_val(&emulator_interface.local_index) as libc::socklen_t,
        ) != 0 {
            return Err(format!("create_multi_socket(): failed to setup IPV6_MULTICAST_IF: {}", std::io::Error::last_os_error()));
        }
    }

    // set IPV6_MULTICAST_HOPS 
    unsafe {
        let ttl: libc::c_int = 2;
        if libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_HOPS,
            &ttl as *const _ as *const libc::c_void,
            std::mem::size_of_val(&ttl) as libc::socklen_t,
        ) != 0 {
            return Err(format!("create_multi_socket(): failed to setup IPV6_MULTICAST_HOPS: {}", std::io::Error::last_os_error()));
        }
    }

    // set IPV6_MULTICAST_LOOP
    unsafe {
        let loopback: libc::c_int = 1;
        if libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_LOOP,
            &loopback as *const _ as *const libc::c_void,
            std::mem::size_of_val(&loopback) as libc::socklen_t,
        ) != 0 {
            return Err(format!("create_multi_socket(): failed to set IPV6_MULTICAST_LOOP: {}", std::io::Error::last_os_error()));
        }
    }

    // set non blocking
    unsafe {
        let flags = fcntl(fd, F_GETFL);
        if flags == -1 {
            return Err(format!("create_multi_socket(): failed to get flags: {}", std::io::Error::last_os_error()));
        }
        let new_flags = if blocking {
            flags & !O_NONBLOCK
        } else {
            flags | O_NONBLOCK
        };
        if fcntl(fd, F_SETFL, new_flags) == -1 {
            return Err(format!("create_multi_socket(): failed to set NON_BLOCKING: {}", std::io::Error::last_os_error()));
        }
    }

    // construct sockaddr_in6 manually using libc
    let sockaddr_in6 = libc::sockaddr_in6 {
        sin6_family: libc::AF_INET6 as libc::sa_family_t,
        sin6_port: socket_addr.port().to_be(),
        sin6_flowinfo: socket_addr.flowinfo(),
        sin6_addr: libc::in6_addr {
            s6_addr: socket_addr.ip().octets(),
        },
        sin6_scope_id: socket_addr.scope_id(),
    };

    // bind the socket to the address
    unsafe {
        if libc::bind(
            fd,
            &sockaddr_in6 as *const _ as *const libc::sockaddr,
            std::mem::size_of_val(&sockaddr_in6) as libc::socklen_t,
        ) != 0 {
            return Err(format!("create_multi_socket(): failed to bind socket: {}", std::io::Error::last_os_error()));
        }
    }

    // convert the raw socket into a Rust UdpSocket
    let socket = unsafe { StdUdpSocket::from_raw_fd(fd) };

    // convert to tokio UdpSocket for async operations
    match TokioUdpsocket::from_std(socket) {
        Ok(tokio_socket) =>  {
            Ok(tokio_socket)
        },
        Err(err) => { 
            Err(format!("create_multi_socket(): failed to convert UDP socket to Tokio socket: {}", err)) 
        }
    }

}


/// Create unicast socket
///     - blocking set to true
///     - setup socket config using libc
///     - return TokioUDPSocket 
pub fn create_unicast_socket(emulator_interface: &mut EmulatorLocalNetwork)
    -> Result<TokioUdpsocket, String>
{

    // unicast socket address - set port to 0 for ephemeral port
    let socket_addr = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, emulator_interface.local_index);

    // create empty IPv6 UDP socket using libc
    let fd = unsafe { libc::socket(libc::AF_INET6, libc::SOCK_DGRAM, 0) };
    if fd < 0 {
        return Err(format!("create_unicast_socket(): failed to create socket: {}", std::io::Error::last_os_error()));
    }
    
    // set SO_REUSEPORT
    unsafe {
        let reuse_port: libc::c_int = 0;
        if libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_REUSEPORT,
            &reuse_port as *const _ as *const libc::c_void,
            std::mem::size_of_val(&reuse_port) as libc::socklen_t
        ) != 0
        {
            return Err(format!("create_unicast_socket(): failed to setup SO_REUSEPORT: {}", std::io::Error::last_os_error()));
        }
    }

    // set SO_REUSEADDR
    unsafe {
        let reuse_addr: libc::c_int = 0;
        if libc::setsockopt(
            fd, 
            libc::SOL_SOCKET, 
            libc::SO_REUSEADDR, 
            &reuse_addr as *const _ as *const libc::c_void, 
            std::mem::size_of_val(&reuse_addr) as libc::socklen_t
        ) != 0 {
            return Err(format!("create_unicast_socket(): failed to setup SO_REUSEADDR: {}", std::io::Error::last_os_error()));
        }
    }

    // set IPV6_MULTICAST_IF
    unsafe {
        if libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_IF,
            &emulator_interface.local_index as *const _ as *const libc::c_void,
            std::mem::size_of_val(&emulator_interface.local_index) as libc::socklen_t,
        ) != 0 {
            return Err(format!("create_unicast_socket(): failed to setup IPV6_MULTICAST_IF: {}", std::io::Error::last_os_error()));
        }
    }

    // set IPV6_MULTICAST_HOPS 
    unsafe {
        let ttl: libc::c_int = 2;
        if libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_HOPS,
            &ttl as *const _ as *const libc::c_void,
            std::mem::size_of_val(&ttl) as libc::socklen_t,
        ) != 0 {
            return Err(format!("create_unicast_socket(): failed to setup IPV6_MULTICAST_HOPS: {}", std::io::Error::last_os_error()));
        }
    }

    // set IPV6_MULTICAST_LOOP
    unsafe {
        let loopback: libc::c_int = 1;
        if libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_LOOP,
            &loopback as *const _ as *const libc::c_void,
            std::mem::size_of_val(&loopback) as libc::socklen_t,
        ) != 0 {
            return Err(format!("create_unicast_socket(): failed to set IPV6_MULTICAST_LOOP: {}", std::io::Error::last_os_error()));
        }
    }

    // set non blocking
    unsafe {
        let flags = fcntl(fd, F_GETFL);
        if flags == -1 {
            return Err(format!("create_unicast_socket(): failed to get flags: {}", std::io::Error::last_os_error()));
        }
        let new_flags = flags | O_NONBLOCK;
        if fcntl(fd, F_SETFL, new_flags) == -1 {
            return Err(format!("create_unicast_socket(): failed to set NON_BLOCKING: {}", std::io::Error::last_os_error()));
        }
    }

    // increase buffer size
    // unnecessary already set to max "cat /proc/sys/net/core/rmem_max"
    // unsafe {
    //     let recv_buffer_size: i32 = 30 * 1024 * 1024; // 4 MB
    //     if libc::setsockopt(
    //         fd,
    //         libc::SOL_SOCKET,
    //         libc::SO_RCVBUF,
    //         &recv_buffer_size as *const _ as *const libc::c_void,
    //         std::mem::size_of_val(&recv_buffer_size) as libc::socklen_t,
    //     ) != 0
    //     {
    //         return Err(format!("create_unicast_socket(): failed to set SO_RCVBUF: {}", std::io::Error::last_os_error()));
    //     }
    // }

    // construct sockaddr_in6 manually using libc
    let sockaddr_in6 = libc::sockaddr_in6 {
        sin6_family: libc::AF_INET6 as libc::sa_family_t,
        sin6_port: socket_addr.port().to_be(),
        sin6_flowinfo: socket_addr.flowinfo(),
        sin6_addr: libc::in6_addr {
            s6_addr: socket_addr.ip().octets(),
        },
        sin6_scope_id: socket_addr.scope_id(),
    };

    // bind the socket to the address
    unsafe {
        if libc::bind(
            fd,
            &sockaddr_in6 as *const _ as *const libc::sockaddr,
            std::mem::size_of_val(&sockaddr_in6) as libc::socklen_t,
        ) != 0 {
            return Err(format!("create_unicast_socket(): failed to bind socket: {}", std::io::Error::last_os_error()));
        }
    }

    // retrieve the ethermal port number after binding
    let mut sockaddr_in6: libc::sockaddr_in6 = unsafe { std::mem::zeroed() };
    let mut addr_len = std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t;
    unsafe {
        if libc::getsockname(
            fd,
            &mut sockaddr_in6 as *mut _ as *mut libc::sockaddr,
            &mut addr_len,
        ) != 0 {
            return Err(format!("create_unicast_socket(): failed to set getsockname(): {}", std::io::Error::last_os_error()));
        }
    }
    let ephermal_port = u16::from_be(sockaddr_in6.sin6_port);
    emulator_interface.set_local_port(ephermal_port);

    // convert the raw socket into a rust UdpSocket
    let socket = unsafe { StdUdpSocket::from_raw_fd(fd) };

    // convert to tokio UdpSocket for async operations
    match TokioUdpsocket::from_std(socket) {
        Ok(tokio_socket) =>  {
            Ok(tokio_socket)
        },
        Err(err) => { 
            Err(format!("create_multi_socket(): failed to convert UDP socket to Tokio socket: {}", err)) 
        }
    }

}