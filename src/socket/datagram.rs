use std::os::fd::OwnedFd;
use std::marker::PhantomData;
use crate::addr::{Domain, ToSockAddr, FromSockAddr};
use crate::error::{SocketError, IoError, errno};

/// A bound datagram socket ready for send/recv.
///
/// Unlike Stream sockets, datagrams don't connect.
/// Each send specifies a destination, each recv tells you the source.
pub struct BoundDatagram<D: Domain> {
	fd: OwnedFd,
	_marker: PhantomData<D>,
}

impl<D: Domain> BoundDatagram<D> {
	pub(crate) fn from_fd(fd: OwnedFd) -> Self {
		Self {
			fd,
			_marker: PhantomData,
		}
	}
	
	#[inline]
	pub fn as_raw_fd(&self) -> libc::c_int {
		use std::os::fd::AsRawFd;
		self.fd.as_raw_fd()
	}
}

impl<D: Domain> BoundDatagram<D>
where
	D::Addr: ToSockAddr,
{
	/// Sends data to a specific address.
	///
	/// Returns the number of bytes sent.
	pub fn send_to(&self, buf: &[u8], addr: &D::Addr) -> std::io::Result<usize> {
		let result = addr.with_raw(|ptr, len| unsafe {
			libc::sendto(
				self.as_raw_fd(),
				buf.as_ptr() as *const libc::c_void,
				buf.len(),
				0,
				ptr,
				len,
			)
		});

		match result {
			Some(n) if n >= 0 => Ok(n as usize),
			Some(_) => Err(IoError::Write { errno: errno() }.into()),
			None => Err(SocketError::InvalidAddress { reason: "address too long" }.into()),
		}
	}

	pub fn send_to_with_flags(&self, buf: &[u8], addr: &D::Addr, flags: i32) -> std::io::Result<usize> {
		let result = addr.with_raw(|ptr, len| unsafe {
			libc::sendto(
				self.as_raw_fd(),
				buf.as_ptr() as *const libc::c_void,
				buf.len(),
				flags,
				ptr,
				len,
			)
		});

		match result {
			Some(n) if n >= 0 => Ok(n as usize),
			Some(_) => Err(IoError::Write { errno: errno() }.into()),
			None => Err(SocketError::InvalidAddress { reason: "address too long" }.into()),
		}
	}
	/// Receives data, returning bytes read.
	///
	/// Does not return sender address (simpler API).
	pub fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
		let n = unsafe {
			libc::recvfrom(
				self.as_raw_fd(),
				buf.as_mut_ptr() as *mut libc::c_void,
				buf.len(),
				0,
				std::ptr::null_mut(),
				std::ptr::null_mut(),
			)
		};

		if n == -1 {
			Err(IoError::Read { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}

	pub fn recv_with_flags(&self, buf: &mut [u8], flags: i32) -> std::io::Result<usize> {
		let n = unsafe {
			libc::recvfrom(
				self.as_raw_fd(),
				buf.as_mut_ptr() as *mut libc::c_void,
				buf.len(),
				flags,
				std::ptr::null_mut(),
				std::ptr::null_mut(),
			)
		};

		if n == -1 {
			Err(IoError::Read { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}
	pub fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, D::Addr)>
	where
		D::Addr: FromSockAddr,
	{
		let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
		let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

		let n = unsafe {
			libc::recvfrom(
				self.as_raw_fd(),
				buf.as_mut_ptr() as *mut libc::c_void,
				buf.len(),
				0,
				&mut storage as *mut _ as *mut libc::sockaddr,
				&mut len,
			)
		};

		if n == -1 {
			return Err(IoError::Read { errno: errno() }.into());
		}

		let addr = unsafe {
			D::Addr::from_sockaddr(&storage as *const _ as *const libc::sockaddr, len)
				.ok_or_else(|| SocketError::InvalidAddress { reason: "invalid sender address" })?
		};

		Ok((n as usize, addr))
	}

	pub fn recv_from_with_flags(&self, buf: &mut [u8], flags: i32) -> std::io::Result<(usize, D::Addr)>
	where
		D::Addr: FromSockAddr,
	{
		let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
		let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

		let n = unsafe {
			libc::recvfrom(
				self.as_raw_fd(),
				buf.as_mut_ptr() as *mut libc::c_void,
				buf.len(),
				flags,
				&mut storage as *mut _ as *mut libc::sockaddr,
				&mut len,
			)
		};

		if n == -1 {
			return Err(IoError::Read { errno: errno() }.into());
		}

		let addr = unsafe {
			D::Addr::from_sockaddr(&storage as *const _ as *const libc::sockaddr, len)
				.ok_or_else(|| SocketError::InvalidAddress { reason: "invalid sender address" })?
		};

		Ok((n as usize, addr))
	}

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
		let result = unsafe { libc::fcntl(self.as_raw_fd(), libc::F_SETFL, new_flags) };
		if result == -1 {
			return Err(SocketError::SetOption { errno: errno(), option: "O_NONBLOCK" }.into());
		}
		Ok(())
	}

	pub fn connect(self, addr: D::Addr) -> std::io::Result<ConnectedDatagram<D>>
	where
		D::Addr: std::fmt::Debug,
	{
		let result = addr.with_raw(|ptr, len| unsafe {
			libc::connect(self.as_raw_fd(), ptr, len)
		});

		match result {
			Some(-1) => Err(SocketError::Connect { errno: errno(), addr: format!("{:?}", addr) }.into()),
			Some(_) => Ok(ConnectedDatagram::from_fd(self.fd)),
			None => Err(SocketError::InvalidAddress { reason: "address too long" }.into()),
		}
	}
}

impl<D: Domain> std::os::fd::AsRawFd for BoundDatagram<D> {
	fn as_raw_fd(&self) -> std::os::fd::RawFd {
		self.fd.as_raw_fd()
	}
}

impl<D: Domain> std::os::fd::AsFd for BoundDatagram<D> {
	fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
		self.fd.as_fd()
	}
}

impl<D: Domain> std::os::fd::FromRawFd for BoundDatagram<D> {
	unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
		unsafe { Self::from_fd(OwnedFd::from_raw_fd(fd)) }
	}
}

impl<D: Domain> std::os::fd::IntoRawFd for BoundDatagram<D> {
	fn into_raw_fd(self) -> std::os::fd::RawFd {
		self.fd.into_raw_fd()
	}
}

/// A UDP socket connected to a specific peer.
///
/// Created by BoundDatagram::connect().
/// send()/recv() only communicate with that peer.
//Identical structure to BoundDatagram. Same fd, same marker.
// Why? Because at the kernel level, it's the same socket.
// The only difference is that connect() was called on it.
// The type is what changed, not the data.
pub struct ConnectedDatagram<D: Domain> {
	fd: OwnedFd,
	_marker: PhantomData<D>,
}

impl<D: Domain> ConnectedDatagram<D> {
	pub(crate) fn from_fd(fd: OwnedFd) -> Self {
		Self {
			fd,
			_marker: PhantomData,
		}
	}
	
	#[inline]
	pub fn as_raw_fd(&self) -> libc::c_int {
		use std::os::fd::AsRawFd;
		self.fd.as_raw_fd()
	}
	
	pub fn send(&self, buf: &[u8]) -> std::io::Result<usize> {
		let n = unsafe {
			libc::send(
				self.as_raw_fd(),
				buf.as_ptr() as *const libc::c_void,
				buf.len(),
				0,
			)
		};

		if n == -1 {
			Err(IoError::Write { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}

	pub fn send_with_flags(&self, buf: &[u8], flags: i32) -> std::io::Result<usize> {
		let n = unsafe {
			libc::send(
				self.as_raw_fd(),
				buf.as_ptr() as *const libc::c_void,
				buf.len(),
				flags,
			)
		};

		if n == -1 {
			Err(IoError::Write { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}

	pub fn recv(&self, buf: &mut [u8]) -> std::io::Result<usize> {
		let n = unsafe {
			libc::recv(
				self.as_raw_fd(),
				buf.as_mut_ptr() as *mut libc::c_void,
				buf.len(),
				0,
			)
		};

		if n == -1 {
			Err(IoError::Read { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}

	pub fn recv_with_flags(&self, buf: &mut [u8], flags: i32) -> std::io::Result<usize> {
		let n = unsafe {
			libc::recv(
				self.as_raw_fd(),
				buf.as_mut_ptr() as *mut libc::c_void,
				buf.len(),
				flags,
			)
		};

		if n == -1 {
			Err(IoError::Read { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}

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
		let result = unsafe { libc::fcntl(self.as_raw_fd(), libc::F_SETFL, new_flags) };
		if result == -1 {
			return Err(SocketError::SetOption { errno: errno(), option: "O_NONBLOCK" }.into());
		}
		Ok(())
	}
}

impl<D: Domain> ConnectedDatagram<D>
where
	D::Addr: FromSockAddr,
{
	pub fn peer_addr(&self) -> std::io::Result<D::Addr> {
		let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
		let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

		let result = unsafe {
			libc::getpeername(
				self.as_raw_fd(),
				&mut storage as *mut _ as *mut libc::sockaddr,
				&mut len,
			)
		};

		if result == -1 {
			return Err(SocketError::GetOption { errno: errno(), option: "SO_PEERNAME" }.into());
		}

		unsafe {
			D::Addr::from_sockaddr(&storage as *const _ as *const libc::sockaddr, len)
				.ok_or_else(|| SocketError::InvalidAddress { reason: "invalid address" }.into())
		}
	}

	/// Receives data and returns the sender's address.
	///
	/// On a connected socket, only packets from the connected peer are delivered.
	/// The returned address will always match `peer_addr()`.
	///
	/// Use `recv()` for normal operation. Use this when you need to verify
	/// the source or during connection migration.
	pub fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, D::Addr)> {
		let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
		let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

		let n = unsafe {
			libc::recvfrom(
				self.as_raw_fd(),
				buf.as_mut_ptr() as *mut libc::c_void,
				buf.len(),
				0,
				&mut storage as *mut _ as *mut libc::sockaddr,
				&mut len,
			)
		};

		if n == -1 {
			return Err(IoError::Read { errno: errno() }.into());
		}

		let addr = unsafe {
			D::Addr::from_sockaddr(&storage as *const _ as *const libc::sockaddr, len)
				.ok_or_else(|| SocketError::InvalidAddress { reason: "invalid sender address" })?
		};

		Ok((n as usize, addr))
	}

	pub fn recv_from_with_flags(&self, buf: &mut [u8], flags: i32) -> std::io::Result<(usize, D::Addr)> {
		let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
		let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

		let n = unsafe {
			libc::recvfrom(
				self.as_raw_fd(),
				buf.as_mut_ptr() as *mut libc::c_void,
				buf.len(),
				flags,
				&mut storage as *mut _ as *mut libc::sockaddr,
				&mut len,
			)
		};

		if n == -1 {
			return Err(IoError::Read { errno: errno() }.into());
		}

		let addr = unsafe {
			D::Addr::from_sockaddr(&storage as *const _ as *const libc::sockaddr, len)
				.ok_or_else(|| SocketError::InvalidAddress { reason: "invalid sender address" })?
		};

		Ok((n as usize, addr))
	}
}


impl<D: Domain> ConnectedDatagram<D>
where
	D::Addr: ToSockAddr,
{
	/// Sends data to a specific address, overriding the connected peer.
	///
	/// Primary communication should use `send()`.
	/// Use this only for probing alternate paths (NAT traversal, QUIC migration).
	pub fn send_to(&self, buf: &[u8], addr: &D::Addr) -> std::io::Result<usize> {
		let result = addr.with_raw(|ptr, len| unsafe {
			libc::sendto(
				self.as_raw_fd(),
				buf.as_ptr() as *const libc::c_void,
				buf.len(),
				0,
				ptr,
				len,
			)
		});

		match result {
			Some(n) if n >= 0 => Ok(n as usize),
			Some(_) => Err(IoError::Write { errno: errno() }.into()),
			None => Err(SocketError::InvalidAddress { reason: "address too long" }.into()),
		}
	}

	pub fn send_to_with_flags(&self, buf: &[u8], addr: &D::Addr, flags: i32) -> std::io::Result<usize> {
		let result = addr.with_raw(|ptr, len| unsafe {
			libc::sendto(
				self.as_raw_fd(),
				buf.as_ptr() as *const libc::c_void,
				buf.len(),
				flags,
				ptr,
				len,
			)
		});

		match result {
			Some(n) if n >= 0 => Ok(n as usize),
			Some(_) => Err(IoError::Write { errno: errno() }.into()),
			None => Err(SocketError::InvalidAddress { reason: "address too long" }.into()),
		}
	}
}

impl<D: Domain> std::os::fd::AsRawFd for ConnectedDatagram<D> {
	fn as_raw_fd(&self) -> std::os::fd::RawFd {
		self.fd.as_raw_fd()
	}
}

impl<D: Domain> std::os::fd::AsFd for ConnectedDatagram<D> {
	fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
		self.fd.as_fd()
	}
}

impl<D: Domain> std::os::fd::FromRawFd for ConnectedDatagram<D> {
	unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
		unsafe { Self::from_fd(OwnedFd::from_raw_fd(fd)) }
	}
}

impl<D: Domain> std::os::fd::IntoRawFd for ConnectedDatagram<D> {
	fn into_raw_fd(self) -> std::os::fd::RawFd {
		self.fd.into_raw_fd()
	}
}
/*
 Why would you use this on a connected socket?
QUIC connection migration. Client sends from new IP.
The packet arrives
	Important nuance: On a strictly connected socket,
	kernel drops packets from non-connected addresses.
	So recv_from() would always return the connected peer.
But QUIC needs to receive from new addresses during migration.
So QUIC implementations often use unconnected sockets with
application-level filtering, or they call connect() to the new address after receiving the migration packet.
Still useful to have recv_from() for debugging and verification.

 */
// reason for separte - only used by udp
impl<D: Domain> BoundDatagram<D>
where
	D::Addr: ToSockAddr + FromSockAddr,
{
	/// Sends multiple messages in one syscall.
	///
	/// Returns number of messages successfully sent.
	pub fn sendmmsg(&self, messages: &[SendMsg<D::Addr>]) -> std::io::Result<usize> {
		if messages.is_empty() {
			return Ok(0);
		}
		
		let len = messages.len();
		
		// Build iovecs for each message
		let mut iovecs: Vec<libc::iovec> = messages
			.iter()
			.map(|msg| libc::iovec {
				iov_base: msg.buf.as_ptr() as *mut libc::c_void,
				iov_len: msg.buf.len(),
			})
			.collect();
		
		// Storage for converted addresses
		let mut sockaddrs: Vec<libc::sockaddr_storage> = vec![unsafe { std::mem::zeroed() }; len];
		let mut addr_lens: Vec<libc::socklen_t> = vec![0; len];
		
		// Convert addresses
		for (i, msg) in messages.iter().enumerate() {
			msg.addr.with_raw(|ptr, addr_len| {
				if addr_len as usize > std::mem::size_of::<libc::sockaddr_storage>() {
					return None;
				}
				unsafe {
					std::ptr::copy_nonoverlapping(
						ptr as *const u8,
						&mut sockaddrs[i] as *mut _ as *mut u8,
						addr_len as usize,
					);
				}
				addr_lens[i] = addr_len;
				Some(())
			}).ok_or_else(|| SocketError::InvalidAddress { reason: "address too long" })?;
		}

		// Build mmsghdr array
		let mut hdrs: Vec<libc::mmsghdr> = (0..len)
			.map(|i| {
				let mut hdr: libc::mmsghdr = unsafe { std::mem::zeroed() };
				hdr.msg_hdr.msg_name = &mut sockaddrs[i] as *mut _ as *mut libc::c_void;
				hdr.msg_hdr.msg_namelen = addr_lens[i];
				hdr.msg_hdr.msg_iov = &mut iovecs[i];
				hdr.msg_hdr.msg_iovlen = 1;
				hdr
			})
			.collect();

		let sent = unsafe {
			libc::sendmmsg(
				self.as_raw_fd(),
				hdrs.as_mut_ptr(),
				len as libc::c_uint,
				0,
			)
		};

		if sent == -1 {
			return Err(IoError::Write { errno: errno() }.into());
		}

		Ok(sent as usize)
	}

	/// Receives multiple messages in one syscall.
	///
	/// `bufs` - mutable buffers to receive into
	/// Returns Vec of (bytes_received, sender_address) per message.
	pub fn recvmmsg(&self, bufs: &mut [&mut [u8]]) -> std::io::Result<Vec<(usize, D::Addr)>> {
		if bufs.is_empty() {
			return Ok(Vec::new());
		}

		let len = bufs.len();

		// Storage for sender addresses
		let mut sockaddrs: Vec<libc::sockaddr_storage> = vec![unsafe { std::mem::zeroed() }; len];

		// Build iovecs
		let mut iovecs: Vec<libc::iovec> = bufs
			.iter_mut()
			.map(|buf| libc::iovec {
				iov_base: buf.as_mut_ptr() as *mut libc::c_void,
				iov_len: buf.len(),
			})
			.collect();

		// Build mmsghdr array
		let mut hdrs: Vec<libc::mmsghdr> = (0..len)
			.map(|i| {
				let mut hdr: libc::mmsghdr = unsafe { std::mem::zeroed() };
				hdr.msg_hdr.msg_name = &mut sockaddrs[i] as *mut _ as *mut libc::c_void;
				hdr.msg_hdr.msg_namelen = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
				hdr.msg_hdr.msg_iov = &mut iovecs[i];
				hdr.msg_hdr.msg_iovlen = 1;
				hdr
			})
			.collect();

		let received = unsafe {
			libc::recvmmsg(
				self.as_raw_fd(),
				hdrs.as_mut_ptr(),
				len as libc::c_uint,
				0,
				std::ptr::null_mut(), // no timeout
			)
		};

		if received == -1 {
			return Err(IoError::Read { errno: errno() }.into());
		}

		// Extract results
		let mut results = Vec::with_capacity(received as usize);
		for i in 0..received as usize {
			let bytes = hdrs[i].msg_len as usize;
			let addr = unsafe {
				D::Addr::from_sockaddr(
					&sockaddrs[i] as *const _ as *const libc::sockaddr,
					hdrs[i].msg_hdr.msg_namelen,
				)
					.ok_or_else(|| SocketError::InvalidAddress { reason: "invalid sender address" })?
			};
			results.push((bytes, addr));
		}

		Ok(results)
	}
}

/// A single message to send via sendmmsg.
pub struct SendMsg<'a, A> {
	pub buf: &'a [u8],
	pub addr: &'a A,
}

/// Result of a single message in sendmmsg/recvmmsg.
pub struct MsgResult {
	pub bytes: usize,
}