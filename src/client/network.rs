use std::ffi::CString;

use crate::extc;
use ::libc;
use anyhow::bail;

use super::{Parameter, Session};

pub unsafe fn create_tcp_socket_client(
    session: &mut Session,
    parameter: &Parameter,
) -> anyhow::Result<i32> {
    let mut hints: extc::addrinfo = extc::addrinfo {
        ai_flags: 0,
        ai_family: 0,
        ai_socktype: 0,
        ai_protocol: 0,
        ai_addrlen: 0,
        ai_addr: std::ptr::null_mut::<extc::sockaddr>(),
        ai_canonname: std::ptr::null_mut::<libc::c_char>(),
        ai_next: std::ptr::null_mut::<extc::addrinfo>(),
    };
    let mut info: *mut extc::addrinfo = std::ptr::null_mut::<extc::addrinfo>();
    let mut info_save: *mut extc::addrinfo = std::ptr::null_mut::<extc::addrinfo>();
    let mut buffer: [libc::c_char; 10] = [0; 10];
    let mut socket_fd: libc::c_int = 0;
    let mut yes: libc::c_int = 1 as libc::c_int;
    let mut status: libc::c_int = 0;
    extc::memset(
        &mut hints as *mut extc::addrinfo as *mut libc::c_void,
        0 as libc::c_int,
        ::core::mem::size_of::<extc::addrinfo>() as libc::c_ulong,
    );
    hints.ai_family = if parameter.ipv6_yn as libc::c_int != 0 {
        10 as libc::c_int
    } else {
        2 as libc::c_int
    };
    hints.ai_socktype = extc::SOCK_STREAM as libc::c_int;
    extc::sprintf(
        buffer.as_mut_ptr(),
        b"%d\0" as *const u8 as *const libc::c_char,
        parameter.server_port as libc::c_int,
    );
    let c_server_name = CString::new(parameter.server_name.as_str()).unwrap();
    status = extc::getaddrinfo(
        c_server_name.as_ptr(),
        buffer.as_mut_ptr(),
        &hints,
        &mut info,
    );
    if status != 0 {
        bail!("Error in getting address information for server");
    }
    info_save = info;
    loop {
        socket_fd = extc::socket((*info).ai_family, (*info).ai_socktype, (*info).ai_protocol);
        if socket_fd < 0 as libc::c_int {
            println!("WARNING: Could not create socket");
        } else {
            status = extc::setsockopt(
                socket_fd,
                1 as libc::c_int,
                2 as libc::c_int,
                &mut yes as *mut libc::c_int as *const libc::c_void,
                ::core::mem::size_of::<libc::c_int>() as libc::c_ulong as extc::socklen_t,
            );
            if status < 0 as libc::c_int {
                println!("WARNING: Could not make socket reusable");
                extc::close(socket_fd);
            } else {
                status = extc::setsockopt(
                    socket_fd,
                    extc::IPPROTO_TCP as libc::c_int,
                    1 as libc::c_int,
                    &mut yes as *mut libc::c_int as *const libc::c_void,
                    ::core::mem::size_of::<libc::c_int>() as libc::c_ulong as extc::socklen_t,
                );
                if status < 0 as libc::c_int {
                    println!("WARNING: Could not disable Nagle's algorithm");
                    extc::close(socket_fd);
                } else {
                    status = extc::connect(
                        socket_fd,
                        extc::__CONST_SOCKADDR_ARG {
                            __sockaddr__: (*info).ai_addr,
                        },
                        (*info).ai_addrlen,
                    );
                    if status == 0 as libc::c_int {
                        session.server_address = extc::malloc((*info).ai_addrlen as libc::c_ulong)
                            as *mut extc::sockaddr;
                        session.server_address_length = (*info).ai_addrlen;
                        if (session.server_address).is_null() {
                            panic!("Could not allocate space for server address");
                        }
                        extc::memcpy(
                            session.server_address as *mut libc::c_void,
                            (*info).ai_addr as *const libc::c_void,
                            (*info).ai_addrlen as libc::c_ulong,
                        );
                        break;
                    }
                }
            }
        }
        info = (*info).ai_next;
        if info.is_null() {
            break;
        }
    }
    extc::freeaddrinfo(info_save);
    if info.is_null() {
        bail!("Error in connecting to Tsunami server");
    }
    Ok(socket_fd)
}
pub unsafe fn create_udp_socket_client(parameter: &mut Parameter) -> anyhow::Result<i32> {
    let mut hints: extc::addrinfo = extc::addrinfo {
        ai_flags: 0,
        ai_family: 0,
        ai_socktype: 0,
        ai_protocol: 0,
        ai_addrlen: 0,
        ai_addr: std::ptr::null_mut::<extc::sockaddr>(),
        ai_canonname: std::ptr::null_mut::<libc::c_char>(),
        ai_next: std::ptr::null_mut::<extc::addrinfo>(),
    };
    let mut info: *mut extc::addrinfo = std::ptr::null_mut::<extc::addrinfo>();
    let mut info_save: *mut extc::addrinfo = std::ptr::null_mut::<extc::addrinfo>();
    let mut buffer: [libc::c_char; 10] = [0; 10];
    let mut socket_fd: libc::c_int = 0;
    let mut status: libc::c_int = 0;
    let mut higher_port_attempt: libc::c_int = 0 as libc::c_int;
    extc::memset(
        &mut hints as *mut extc::addrinfo as *mut libc::c_void,
        0 as libc::c_int,
        ::core::mem::size_of::<extc::addrinfo>() as libc::c_ulong,
    );
    hints.ai_flags = 0x1 as libc::c_int;
    hints.ai_family = if parameter.ipv6_yn as libc::c_int != 0 {
        10 as libc::c_int
    } else {
        2 as libc::c_int
    };
    hints.ai_socktype = extc::SOCK_DGRAM as libc::c_int;
    loop {
        extc::sprintf(
            buffer.as_mut_ptr(),
            b"%d\0" as *const u8 as *const libc::c_char,
            parameter.client_port as libc::c_int + higher_port_attempt,
        );
        status = extc::getaddrinfo(
            std::ptr::null::<libc::c_char>(),
            buffer.as_mut_ptr(),
            &hints,
            &mut info,
        );
        if status != 0 {
            bail!("Error in getting address information");
        }
        info_save = info;
        loop {
            socket_fd = extc::socket((*info).ai_family, (*info).ai_socktype, (*info).ai_protocol);
            if socket_fd >= 0 as libc::c_int {
                status = extc::setsockopt(
                    socket_fd,
                    1 as libc::c_int,
                    8 as libc::c_int,
                    &parameter.udp_buffer as *const u32 as *const libc::c_void,
                    ::core::mem::size_of::<u32>() as libc::c_ulong as extc::socklen_t,
                );
                if status < 0 as libc::c_int {
                    println!("WARNING: Error in resizing UDP receive buffer");
                }
                status = extc::bind(
                    socket_fd,
                    extc::__CONST_SOCKADDR_ARG {
                        __sockaddr__: (*info).ai_addr,
                    },
                    (*info).ai_addrlen,
                );
                if status == 0 as libc::c_int {
                    parameter.client_port =
                        extc::__bswap_16((*((*info).ai_addr as *mut extc::sockaddr_in)).sin_port);
                    extc::fprintf(
                        extc::stderr,
                        b"Receiving data on UDP port %d\n\0" as *const u8 as *const libc::c_char,
                        parameter.client_port as libc::c_int,
                    );
                    break;
                }
            }
            info = (*info).ai_next;
            if info.is_null() {
                break;
            }
        }
        extc::freeaddrinfo(info_save);
        higher_port_attempt += 1;
        if !(higher_port_attempt < 256 as libc::c_int && info.is_null()) {
            break;
        }
    }
    if higher_port_attempt > 1 as libc::c_int {
        extc::fprintf(
            extc::stderr,
            b"Warning: there are %d other Tsunami clients running\n\0" as *const u8
                as *const libc::c_char,
            higher_port_attempt - 1 as libc::c_int,
        );
    }
    if info.is_null() {
        bail!("Error in creating UDP socket");
    }
    Ok(socket_fd)
}
