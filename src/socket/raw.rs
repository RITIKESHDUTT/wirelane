use crate::socket::pending::PendingConnect;
use crate::Datagram;
use crate::socket::datagram::BoundDatagram;
use crate::socket::Stream;
use crate::socket::stream::ConnectedStream;
use crate::addr::ToSockAddr;
use std::os::fd::{OwnedFd, FromRawFd};
use std::marker::PhantomData;
use crate::addr::Domain;
use crate::error::{SocketError, errno};
use super::SockType;
use super::bound::BoundSocket;

/// A raw socket that has been created but not yet bound or connected.
///
/// This is the starting point for all socket operations.
/// Use `.bind()` to become a listener or datagram socket.
/// Use `.connect()` to become a connected stream.
pub struct RawSocket<D: Domain, T: SockType> {
	fd: OwnedFd,
	_marker: PhantomData<(D, T)>,
}
/*
 ---
Why PhantomData?
We don't store D or T as actual data — they're zero-sized.
But Rust needs to know the struct "uses" these types for:
- Lifetime checking
- Drop behavior
- Variance
PhantomData<(D, T)> says "pretend I hold these types" without storing anything.
*/

impl <D: Domain, T: SockType> RawSocket<D, T> {
	/// Creates a new raw socket.
	///
	/// Calls the `socket()` syscall with the appropriate domain and type.
	/// The socket is created with `SOCK_CLOEXEC` (close on exec).
	pub fn new() -> std::io::Result<Self> {
		let fd = unsafe {
			libc::socket(D::raw(), T::raw() | libc::SOCK_CLOEXEC, 0)
		};
		if fd == -1 {
			return Err(SocketError::Create { errno: errno() }.into());
		}
		let fd = unsafe { OwnedFd::from_raw_fd(fd) };

		Ok(Self {
			fd,
			_marker: PhantomData,
		})
	}
	/// Returns the raw file descriptor.
	///
	/// Used internally for syscalls. Does not transfer ownership.
	#[inline]
	pub fn as_raw_fd(&self) -> libc::c_int {
		use std::os::fd::AsRawFd;
		self.fd.as_raw_fd()
	}
	
	/// Sets the socket to non-blocking mode.
	///
	/// Required for use with epoll/io_uring.
	pub fn set_nonblocking(&self, nonblocking: bool) -> std::io::Result<()> {
		let flags = unsafe { libc::fcntl(self.as_raw_fd(), libc::F_GETFL) };

		if flags == -1 {
			return Err(SocketError::GetOption { errno: errno(), option: "F_GETFL" }.into());
		}

		let new_flags = if nonblocking {
			flags | libc::O_NONBLOCK
		} else {
			flags & !libc::O_NONBLOCK
		};

		let result = unsafe {
			libc::fcntl(self.as_raw_fd(), libc::F_SETFL, new_flags)
		};

		if result == -1 {
			return Err(SocketError::SetOption { errno: errno(), option: "O_NONBLOCK" }.into());
		}

		Ok(())
	}
	
	/// Binds the socket to an address.
	///
	/// Consumes self, returns BoundSocket.
	/// The address type is determined by the Domain:
	/// - Ipv4 → SocketAddrV4
	/// - Ipv6 → SocketAddrV6
	/// - Unix → UnixAddr
	pub fn bind(self, addr: D::Addr) -> std::io::Result<BoundSocket<D, T>>
	where
		D::Addr: ToSockAddr + std::fmt::Debug,
	{
		let result = addr.with_raw(|ptr, len| unsafe {
			libc::bind(self.as_raw_fd(), ptr, len)
		});

		match result {
			Some(-1) => Err(SocketError::Bind {
				errno: errno(),
				addr: format!("{:?}", addr),
			}.into()),
			Some(_) => Ok(BoundSocket::from_fd(self.into_fd())),
			None => Err(SocketError::InvalidAddress {
				reason: "address too long",
			}.into()),
		}
	}
	
	pub(crate) fn into_fd(self) -> OwnedFd {
		self.fd
	}
}
/*
---
What this does:
1. D::raw() — gets AF_INET, AF_INET6, or AF_UNIX
2. T::raw() — gets SOCK_STREAM or SOCK_DGRAM
3. SOCK_CLOEXEC — closes fd if process calls exec()
4. Returns OwnedFd — auto-closes on drop

asrawfd -
Later syscalls (bind, connect, setsockopt) need the raw fd number.
This borrows it without giving up ownership.
 */

/*
what bind() needs to do:
1. Take self (consume RawSocket)
2. Take addr: D::Addr
3. Convert addr to libc::sockaddr_in / sockaddr_in6 / sockaddr_un
4. Call libc::bind()
5. Return the next state type
*/
impl<D: Domain> RawSocket<D, Stream> {
	/// Connects to a remote address.
	///
	/// For clients — establishes connection to a server.
	/// Consumes self, returns a connected stream.
	pub fn connect(self, addr: D::Addr) -> std::io::Result<ConnectedStream<D>>
	where
		D::Addr: ToSockAddr + std::fmt::Debug,
	{
		let result = addr.with_raw(|ptr, len| unsafe {
			libc::connect(self.as_raw_fd(), ptr, len)
		});

		match result {
			Some(-1) => Err(SocketError::Connect {
				errno: errno(),
				addr: format!("{:?}", addr),
			}.into()),
			Some(_) => Ok(ConnectedStream::from_fd(self.into_fd())),
			None => Err(SocketError::InvalidAddress {
				reason: "address too long",
			}.into()),
		}
	}

	/// Starts a non-blocking connection.
	///
	/// Sets the socket to non-blocking, initiates connect, returns immediately.
	/// Use epoll/io_uring to wait for writability, then check `take_error()`.
	pub fn connect_nonblocking(self, addr: D::Addr) -> std::io::Result<PendingConnect<D>>
	where
		D::Addr: ToSockAddr + std::fmt::Debug + Clone,
	{
		// Ensure non-blocking
		self.set_nonblocking(true)?;

		let result = addr.with_raw(|ptr, len| unsafe {
			libc::connect(self.as_raw_fd(), ptr, len)
		});

		match result {
			Some(0) => {
				// Immediate success (rare, but possible on localhost)
				Ok(PendingConnect::from_fd(self.into_fd()))
			}
			Some(-1) => {
				let e = errno();
				if e == libc::EINPROGRESS {
					// Expected: connection in progress
					Ok(PendingConnect::from_fd(self.into_fd()))
				} else {
					Err(SocketError::Connect {
						errno: e,
						addr: format!("{:?}", addr),
					}.into())
				}
			}
			Some(_) => unreachable!("connect() returned unexpected value"),
			None => Err(SocketError::InvalidAddress {
				reason: "address too long",
			}.into()),
		}
	}
}

impl<D: Domain> RawSocket<D, Datagram> {
	/// Binds a datagram socket to an address.
	///
	/// Returns BoundDatagram ready for send_to/recv.
	pub fn bind_datagram(self, addr: D::Addr) -> std::io::Result<BoundDatagram<D>>
	where
		D::Addr: ToSockAddr + std::fmt::Debug,
	{
		let result = addr.with_raw(|ptr, len| unsafe {
			libc::bind(self.as_raw_fd(), ptr, len)
		});

		match result {
			Some(-1) => Err(SocketError::Bind {
				errno: errno(),
				addr: format!("{:?}", addr),
			}.into()),
			Some(_) => Ok(BoundDatagram::from_fd(self.into_fd())),
			None => Err(SocketError::InvalidAddress {
				reason: "address too long",
			}.into()),
		}
	}
}
impl<D: Domain, T: SockType> std::os::fd::AsRawFd for RawSocket<D, T> {
	fn as_raw_fd(&self) -> std::os::fd::RawFd {
		self.fd.as_raw_fd()
	}
}

impl<D: Domain, T: SockType> std::os::fd::AsFd for RawSocket<D, T> {
	fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
		self.fd.as_fd()
	}
}

impl<D: Domain, T: SockType> std::os::fd::FromRawFd for RawSocket<D, T> {
	unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
		unsafe { Self { fd: OwnedFd::from_raw_fd(fd), _marker: PhantomData } }
	}
}

impl<D: Domain, T: SockType> std::os::fd::IntoRawFd for RawSocket<D, T> {
	fn into_raw_fd(self) -> std::os::fd::RawFd {
		self.fd.into_raw_fd()
	}
}