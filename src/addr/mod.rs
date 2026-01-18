//! Address families and related types.
//!
//! This module defines the three address families supported:
//! - `Ipv4` — Internet Protocol version 4
//! - `Ipv6` — Internet Protocol version 6
//! - `Unix` — Unix domain sockets (local only)

mod ipv4;
mod ipv6;
mod unix;
pub use self::ipv4::{Ipv4, SocketAddrV4};
pub use self::ipv6::{Ipv6, SocketAddrV6};
pub use self::unix::{Unix, UnixAddr};

/// Trait for address family markers.
///
/// Each type implementing this trait represents an address family
/// that can be passed to the `socket()` syscall.
pub trait Domain {
	/// Returns the libc constant for this address family.
	type Addr; // type Addr; — this says "every Domain must specify its address type."
	fn raw() -> libc::c_int;
}

/*
Now we have all address conversion methods. The next step is to implement RawSocket::bind() which
will:
	1. Take an address
	2. Call libc::bind()
	3. Consume self and return BoundSocket

But there's a challenge: the address type depends on the Domain. RawSocket<Ipv4, Stream> should
accept SocketAddrV4, while RawSocket<Ipv6, Stream> should accept SocketAddrV6.

We already have this set up with the associated type Domain::Addr. So bind() should take D::Addr as
the address parameter.
 */


/// Trait for address types that can be converted to raw sockaddr for syscalls.
pub trait ToSockAddr {
	/// Calls the provided closure with a pointer to the raw sockaddr and its size.
	/// Returns None if the address is invalid (e.g., path too long for Unix).
	fn with_raw<F, R>(&self, f: F) -> Option<R>
	where
		F: FnOnce(*const libc::sockaddr, libc::socklen_t) -> R;
}
/*
- <F, R> — two generic type parameters (placeholders)
  - &self — takes a reference to the struct
  - f: F — a parameter named f of type F (we'll define what F is below)
  - -> Option<R> — returns Option containing type R
So F and R are unknowns. The where clause constrains what F can be:
This says: "F must be something callable that takes two arguments and returns R."
FnOnce means "a function or closure that can be called once."
The part in parentheses is its signature:
- First argument: *const libc::sockaddr (a pointer)
  - Second argument: libc::socklen_t (a size)
  - Returns: R (whatever type we chose for R)

  A closure that fits this:
  |ptr, len| {
      libc::bind(fd, ptr, len)  // returns c_int
  }
Here R becomes c_int because that's what the closure returns.

Why a closure pattern? Because sockaddr_in, sockaddr_in6, and sockaddr_un are different sizes.
We can't return a pointer to a local variable.
The closure lets us use the stack-allocated struct while it's still alive.
 */
/*
- FnOnce — can be called once. May consume captured variables.
- FnMut — can be called multiple times. May mutate captured variables.
- Fn — can be called multiple times. Only reads captured variables.
 */

/// Trait for address types that can be created from raw sockaddr.
pub trait FromSockAddr: Sized {
	/// Creates address from raw sockaddr storage.
	///
	/// # Safety
	/// The sockaddr must be of the correct family for this type.
	unsafe fn from_sockaddr(addr: *const libc::sockaddr, len: libc::socklen_t) -> Option<Self>;
}

impl FromSockAddr for SocketAddrV4 {
	unsafe fn from_sockaddr(addr: *const libc::sockaddr, len: libc::socklen_t) -> Option<Self> {
		if len < std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t {
			return None;
		}
		let raw = unsafe { &*(addr as *const libc::sockaddr_in) };
		Some(Self::from_raw(raw))
	}
}

impl FromSockAddr for SocketAddrV6 {
	unsafe fn from_sockaddr(addr: *const libc::sockaddr, len: libc::socklen_t) -> Option<Self> {
		if len < std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t {
			return None;
		}
		let raw =unsafe { &*(addr as *const libc::sockaddr_in6) };
		Some(Self::from_raw(raw))
	}
}

impl FromSockAddr for UnixAddr {
	unsafe fn from_sockaddr(addr: *const libc::sockaddr, len: libc::socklen_t) -> Option<Self> {
		if len < std::mem::size_of::<libc::sa_family_t>() as libc::socklen_t {
			return None;
		}
		let raw = unsafe {&*(addr as *const libc::sockaddr_un) };
		Some(Self::from_raw(raw))
	}
}