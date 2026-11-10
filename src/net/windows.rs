//! Windows network state and browser processes through iphlpapi, toolhelp
//! and ntdll. Every call works without administrator rights.

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::net::Ipv4Addr;
use std::ptr;

use windows_sys::Wdk::System::Threading::NtQueryInformationProcess;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_BUFFER_OVERFLOW, ERROR_INSUFFICIENT_BUFFER, HANDLE, INVALID_HANDLE_VALUE,
    NO_ERROR, UNICODE_STRING,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses,
    GetBestRoute2, GetExtendedTcpTable, IP_ADAPTER_ADDRESSES_LH, MIB_IPFORWARD_ROW2,
    MIB_TCP_STATE_ESTAB, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
};
use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows_sys::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN, SOCKADDR_INET};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, TerminateProcess,
};

use super::{Process, TcpConn};
use crate::diagnose::{Adapter, Route};

/// `PROCESSINFOCLASS::ProcessCommandLineInformation` (Windows 8.1+).
const PROCESS_COMMAND_LINE_INFORMATION: i32 = 60;

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: The handle came from a successful Open*/Create* call and is closed once.
        unsafe { CloseHandle(self.0) };
    }
}

fn wide_to_string(w: &[u16]) -> String {
    let end = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..end])
}

fn ipv4(addr: &SOCKADDR_IN) -> Ipv4Addr {
    // SAFETY: S_addr is the plain u32 view of the union; it is always valid.
    Ipv4Addr::from(u32::from_be(unsafe { addr.sin_addr.S_un.S_addr }))
}

/// Which route and source address Windows would use for `dst`.
pub fn best_route(dst: Ipv4Addr) -> Option<Route> {
    // SAFETY: Zeroed SOCKADDR_INET/MIB_IPFORWARD_ROW2 are valid empty values; we fill the IPv4
    // arm and hand GetBestRoute2 pointers that live for the call.
    unsafe {
        let mut d: SOCKADDR_INET = zeroed();
        d.Ipv4.sin_family = AF_INET;
        d.Ipv4.sin_addr.S_un.S_addr = u32::from(dst).to_be();
        let mut row: MIB_IPFORWARD_ROW2 = zeroed();
        let mut src: SOCKADDR_INET = zeroed();
        if GetBestRoute2(ptr::null(), 0, ptr::null(), &d, 0, &mut row, &mut src) != NO_ERROR {
            return None;
        }
        Some(Route {
            if_index: row.InterfaceIndex,
            prefix_len: row.DestinationPrefix.PrefixLength,
            source: (src.si_family == AF_INET).then(|| ipv4(&src.Ipv4)),
        })
    }
}

/// All established IPv4 TCP connections with owner PIDs.
pub fn established_tcp() -> Result<Vec<TcpConn>, String> {
    let mut size: u32 = 64 * 1024;
    for _ in 0..5 {
        let mut buf = vec![0u64; (size as usize).div_ceil(8)];
        // SAFETY: `buf` holds `size` bytes, 8-aligned, and outlives the call.
        let r = unsafe {
            GetExtendedTcpTable(
                buf.as_mut_ptr().cast::<c_void>(),
                &mut size,
                0,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };
        if r == ERROR_INSUFFICIENT_BUFFER {
            size += 16 * 1024;
            continue;
        }
        if r != NO_ERROR {
            return Err(format!("GetExtendedTcpTable: error {r}"));
        }
        let base = buf.as_ptr().cast::<u8>();
        // SAFETY: The table starts with dwNumEntries (u32) followed by that many rows, as
        // GetExtendedTcpTable just wrote into `buf`. Rows are u32-aligned after the count.
        let rows = unsafe {
            let n = *(base as *const u32) as usize;
            std::slice::from_raw_parts(base.add(4).cast::<MIB_TCPROW_OWNER_PID>(), n)
        };
        let port = |p: u32| u16::from_be(p as u16);
        return Ok(rows
            .iter()
            .filter(|r| r.dwState == MIB_TCP_STATE_ESTAB as u32)
            .map(|r| TcpConn {
                pid: r.dwOwningPid,
                local: Ipv4Addr::from(u32::from_be(r.dwLocalAddr)),
                local_port: port(r.dwLocalPort),
                remote: Ipv4Addr::from(u32::from_be(r.dwRemoteAddr)),
                remote_port: port(r.dwRemotePort),
            })
            .collect());
    }
    Err("GetExtendedTcpTable: the table keeps growing".into())
}

/// Network adapters with their IPv4 addresses.
pub fn adapters() -> Result<Vec<Adapter>, String> {
    let mut size: u32 = 32 * 1024;
    for _ in 0..5 {
        let mut buf = vec![0u64; (size as usize).div_ceil(8)];
        let first = buf.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        // SAFETY: `buf` holds `size` bytes, 8-aligned, and outlives every pointer read below.
        let r = unsafe {
            GetAdaptersAddresses(
                AF_INET as u32,
                GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER,
                ptr::null(),
                first,
                &mut size,
            )
        };
        if r == ERROR_BUFFER_OVERFLOW {
            continue;
        }
        if r != NO_ERROR {
            return Err(format!("GetAdaptersAddresses: error {r}"));
        }
        let mut out = Vec::new();
        let mut a = first;
        // SAFETY: A linked list inside `buf`, terminated by a null `Next`.
        unsafe {
            while !a.is_null() {
                let ad = &*a;
                let mut addrs = Vec::new();
                let mut u = ad.FirstUnicastAddress;
                while !u.is_null() {
                    let sa = (*u).Address.lpSockaddr;
                    if !sa.is_null() && (*sa).sa_family == AF_INET {
                        addrs.push(ipv4(&*(sa as *const SOCKADDR_IN)));
                    }
                    u = (*u).Next;
                }
                let desc = if ad.Description.is_null() {
                    String::new()
                } else {
                    let mut len = 0;
                    while *ad.Description.add(len) != 0 {
                        len += 1;
                    }
                    String::from_utf16_lossy(std::slice::from_raw_parts(ad.Description, len))
                };
                out.push(Adapter {
                    index: ad.Anonymous1.Anonymous.IfIndex,
                    description: desc,
                    up: ad.OperStatus == IfOperStatusUp,
                    addrs,
                });
                a = ad.Next;
            }
        }
        return Ok(out);
    }
    Err("GetAdaptersAddresses: the buffer keeps growing".into())
}

/// Running processes whose executable name matches one of `names`
/// (case-insensitive); all processes when `names` is empty.
pub fn processes(names: &[String]) -> Result<Vec<Process>, String> {
    // SAFETY: Toolhelp snapshot APIs with a correctly sized PROCESSENTRY32W.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return Err("CreateToolhelp32Snapshot failed".into());
        }
        let _snap = Handle(snap);
        let mut e: PROCESSENTRY32W = zeroed();
        e.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut out = Vec::new();
        let mut ok = Process32FirstW(snap, &mut e);
        while ok != 0 {
            let name = wide_to_string(&e.szExeFile);
            if names.is_empty() || names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
                out.push(Process {
                    pid: e.th32ProcessID,
                    name,
                });
            }
            ok = Process32NextW(snap, &mut e);
        }
        Ok(out)
    }
}

/// Another process's command line. Needs only
/// PROCESS_QUERY_LIMITED_INFORMATION, which a user has on their own processes.
pub fn command_line(pid: u32) -> Option<String> {
    // SAFETY: The handle is checked and closed; the buffer is 8-aligned, sized as the call
    // asks, and starts with the UNICODE_STRING whose Buffer points into it.
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let _h = Handle(h);
        let mut size: u32 = 4096;
        for _ in 0..4 {
            let mut buf = vec![0u64; (size as usize).div_ceil(8)];
            let mut ret: u32 = 0;
            let status = NtQueryInformationProcess(
                h,
                PROCESS_COMMAND_LINE_INFORMATION,
                buf.as_mut_ptr().cast(),
                size,
                &mut ret,
            );
            if status >= 0 {
                let us = &*(buf.as_ptr() as *const UNICODE_STRING);
                if us.Buffer.is_null() {
                    return Some(String::new());
                }
                return Some(String::from_utf16_lossy(std::slice::from_raw_parts(
                    us.Buffer,
                    us.Length as usize / 2,
                )));
            }
            if ret <= size {
                return None;
            }
            size = ret;
        }
        None
    }
}

/// Ends a process the current user owns.
pub fn terminate(pid: u32) -> Result<(), String> {
    // SAFETY: The handle is checked and closed.
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if h.is_null() {
            return Err(format!(
                "OpenProcess({pid}) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let _h = Handle(h);
        if TerminateProcess(h, 1) == 0 {
            return Err(format!(
                "TerminateProcess({pid}) failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
}
