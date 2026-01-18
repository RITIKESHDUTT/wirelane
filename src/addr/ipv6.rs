use crate::{Domain};
use crate::addr::ToSockAddr;

/// IPv6 address family marker.
///
/// Sockets with this domain use 128-bit addresses (e.g., ::1).
pub struct Ipv6;

impl Domain for Ipv6 {
	type Addr = SocketAddrV6;
	
	#[inline]
	fn raw() -> libc::c_int {
		libc::AF_INET6
	}
}




/// IPv6 socket address (IP + port + scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV6 {
	ip: [u8; 16],
	port: u16,
	/// Scope ID for link-local addresses (identifies network interface).
	/// Usually 0 unless using link-local addresses like fe80::.
	scope_id: u32,
}

impl SocketAddrV6 {
	/// Creates a new IPv6 address.
	pub fn new(ip: [u8; 16], port: u16) -> Self {
		Self { ip, port, scope_id: 0 }
	}
	
	/// Creates with explicit scope ID.
	///
	/// Use for link-local addresses (fe80::) where you need to specify the interface.
	pub fn with_scope(ip: [u8; 16], port: u16, scope_id: u32) -> Self {
		Self { ip, port, scope_id }
	}
	
	/// Returns the IP bytes.
	pub fn ip(&self) -> [u8; 16] {
		self.ip
	}
	
	/// Returns the port.
	pub fn port(&self) -> u16 {
		self.port
	}
	
	/// Returns the scope ID.
	pub fn scope_id(&self) -> u32 {
		self.scope_id
	}
	
	/// Converts to the raw sockaddr_in6 for syscalls.
	pub(crate) fn to_raw(&self) -> libc::sockaddr_in6 {
		libc::sockaddr_in6 {
			sin6_family: libc::AF_INET6 as libc::sa_family_t,
			sin6_port: self.port.to_be(),
			sin6_flowinfo: 0,
			sin6_addr: libc::in6_addr {
				s6_addr: self.ip,
			},
			sin6_scope_id: self.scope_id,
		}
	}
	
	/// Creates from raw sockaddr_in6.
	pub(crate) fn from_raw(raw: &libc::sockaddr_in6) -> Self {
		Self {
			ip: raw.sin6_addr.s6_addr,
			port: u16::from_be(raw.sin6_port),
			scope_id: raw.sin6_scope_id,
		}
	}
}

/*
explaination for above-
 What this does:

  - Each impl provides the constant for socket() syscall
  - #[inline] — compiler will replace the function call with the constant directly
  - At runtime: zero cost, just the number 2 or 10 or 1

  ---
  The mapping:
  ┌────────┬──────────┬───────────────┐
  │ Marker │ Constant │ Value (Linux) │
  ├────────┼──────────┼───────────────┤
  │ Ipv4   │ AF_INET  │ 2             │
  ├────────┼──────────┼───────────────┤
  │ Ipv6   │ AF_INET6 │ 10            │
  ├────────┼──────────┼───────────────┤
  │ Unix   │ AF_UNIX  │ 1             │
  └────────┴──────────┴───────────────┘

 */


/*
Same pattern like V4 ,but bigger IP:
- 16 bytes for IP address
- 2 bytes for port
- Total: 18 bytes
SocketAddrV6::to_raw();
Two extra fields compared to IPv4:
- sin6_flowinfo: Traffic class and flow label (usually 0)
- sin6_scope_id: Link-local scope (usually 0 unless you're dealing with link-local addresses like
 fe80::)
*/

impl ToSockAddr for SocketAddrV6 {
	fn with_raw<F, R>(&self, f: F) -> Option<R>
	where
		F: FnOnce(*const libc::sockaddr, libc::socklen_t) -> R,
	{
		let raw = self.to_raw();
		let ptr = &raw as *const _ as *const libc::sockaddr;
		let len = std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t;
		Some(f(ptr, len))
	}
}