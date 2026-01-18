use crate::{Domain};
use crate::addr::ToSockAddr;

/// IPv4 address family marker.
///
/// Sockets with this domain use 32-bit addresses (e.g., 192.168.1.1).
pub struct Ipv4;

impl Domain for Ipv4 {
	type Addr = SocketAddrV4;
	
	#[inline]
	fn raw() -> libc::c_int {
		libc::AF_INET
	}
}

/// IPv4 socket address (IP + port).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV4 {
	ip: [u8; 4],
	port: u16,
}

impl SocketAddrV4 {
	/// Creates a new IPv4 address.
	pub fn new(ip: [u8; 4], port: u16) -> Self {
		Self { ip, port }
	}
	
	/// Creates from an IP tuple and port.
	/// Example: `SocketAddrV4::from((192, 168, 1, 1), 8080)`
	pub fn from(ip: (u8, u8, u8, u8), port: u16) -> Self {
		Self {
			ip: [ip.0, ip.1, ip.2, ip.3],
			port,
		}
	}
	
	/// Creates from raw sockaddr_in.
	pub(crate) fn from_raw(raw: &libc::sockaddr_in) -> Self {
		Self {
			ip: raw.sin_addr.s_addr.to_ne_bytes(),
			port: u16::from_be(raw.sin_port),
		}
	}
	
	/// Returns the IP bytes.
	pub fn ip(&self) -> [u8; 4] {
		self.ip
	}
	
	/// Returns the port.
	pub fn port(&self) -> u16 {
		self.port
	}
	
	/// Converts to the raw sockaddr_in for syscalls.
	pub(crate) fn to_raw(&self) -> libc::sockaddr_in {
		libc::sockaddr_in {
			sin_family: libc::AF_INET as libc::sa_family_t,
			sin_port: self.port.to_be(),
			sin_addr: libc::in_addr {
				s_addr: u32::from_be_bytes(self.ip).to_be(),
			},
			sin_zero: [0; 8],
		}
	}
	
}


impl ToSockAddr for SocketAddrV4 {
	fn with_raw<F, R>(&self, f: F) -> Option<R>
	where
		F: FnOnce(*const libc::sockaddr, libc::socklen_t) -> R,
	{
		let raw = self.to_raw();  // sockaddr_in lives on THIS stack frame
		let ptr = &raw as *const _ as *const libc::sockaddr;
		let len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
		Some(f(ptr, len))  // call the closure while raw is still alive
	}  // raw dropped here, but closure already finished
}


/*
What this is:
- 4 bytes for IP address
- 2 bytes for port
- Total: 6 bytes
SocketAddrV4::to_raw() =
What each field means:
  - sin_family: Address family (AF_INET = 2 for IPv4)
  - sin_port: Port in network byte order (big-endian)
  - sin_addr: IP address in network byte order
  - sin_zero: Padding to match sockaddr size (historical artifact)
 */