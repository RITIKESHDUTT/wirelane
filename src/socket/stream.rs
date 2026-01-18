use std::io::IoSlice;
use std::io::IoSliceMut;
use std::os::fd::IntoRawFd;
use std::os::fd::RawFd;
use std::os::fd::FromRawFd;
use crate::addr::FromSockAddr;
use std::os::fd::OwnedFd;
use std::marker::PhantomData;
use crate::addr::Domain;
use crate::error::{SocketError, IoError, errno};

/// A connected stream socket.
///
/// Represents an established connection — ready for read/write.
/// Created by Listener::accept() (server) or RawSocket::connect() (client).
pub struct ConnectedStream<D: Domain> {
	fd: OwnedFd,
	_marker: PhantomData<D>,
}

impl<D: Domain> ConnectedStream<D> {
	/// Creates from an OwnedFd.
	pub(crate) fn from_fd(fd: OwnedFd) -> Self {
		Self {
			fd,
			_marker: PhantomData,
		}
	}
	/// Returns the raw file descriptor.
	#[inline]
	pub fn as_raw_fd(&self) -> libc::c_int {
		use std::os::fd::AsRawFd;
		self.fd.as_raw_fd()
	}
	pub fn read(&self, buf: &mut [u8]) -> std::io::Result<usize> {
		let n = unsafe {
			libc::read(
				self.as_raw_fd(),
				buf.as_mut_ptr() as *mut libc::c_void,
				buf.len(),
			)
		};

		if n == -1 {
			Err(IoError::Read { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}

	pub fn write(&self, buf: &[u8]) -> std::io::Result<usize> {
		let n = unsafe {
			libc::write(
				self.as_raw_fd(),
				buf.as_ptr() as *const libc::c_void,
				buf.len(),
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

	pub fn readv(&self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
		let n = unsafe {
			libc::readv(
				self.as_raw_fd(),
				bufs.as_ptr() as *const libc::iovec,
				bufs.len() as libc::c_int,
			)
		};

		if n == -1 {
			Err(IoError::Read { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}

	pub fn writev(&self, bufs: &[IoSlice<'_>]) -> std::io::Result<usize> {
		let n = unsafe {
			libc::writev(
				self.as_raw_fd(),
				bufs.as_ptr() as *const libc::iovec,
				bufs.len() as libc::c_int,
			)
		};

		if n == -1 {
			Err(IoError::Write { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}

	/// Zero-copy file transfer to socket.
	///
	/// Transfers `count` bytes from `file` starting at `offset`.
	/// If `offset` is None, uses file's current position.
	/// Returns number of bytes sent.
	pub fn sendfile<F: std::os::fd::AsRawFd>(
		&self,
		file: &F,
		offset: Option<&mut i64>,
		count: usize,
	) -> std::io::Result<usize> {
		let offset_ptr = match offset {
			Some(off) => off as *mut i64,
			None => std::ptr::null_mut(),
		};

		let n = unsafe {
			libc::sendfile(
				self.as_raw_fd(),
				file.as_raw_fd(),
				offset_ptr,
				count,
			)
		};

		if n == -1 {
			Err(IoError::Write { errno: errno() }.into())
		} else {
			Ok(n as usize)
		}
	}
}

impl<D: Domain> std::os::fd::AsRawFd for ConnectedStream<D> {
	fn as_raw_fd(&self) -> std::os::fd::RawFd {
		self.fd.as_raw_fd()
	}
}

impl<D: Domain> std::io::Read for ConnectedStream<D> {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		ConnectedStream::read(self, buf)
	}
}
impl<D: Domain> std::os::fd::AsFd for ConnectedStream<D> {
	fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
		self.fd.as_fd()
	}
}

impl<D: Domain> std::io::Write for ConnectedStream<D> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		ConnectedStream::write(self, buf)
	}
	
	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())  // TCP doesn't at this level
	}
}

impl<D: Domain> ConnectedStream<D>
where
	D::Addr: FromSockAddr,
{
	/// Returns the remote address of this connection.
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

	/// Returns the local address of this connection.
	pub fn local_addr(&self) -> std::io::Result<D::Addr> {
		let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
		let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

		let result = unsafe {
			libc::getsockname(
				self.as_raw_fd(),
				&mut storage as *mut _ as *mut libc::sockaddr,
				&mut len,
			)
		};

		if result == -1 {
			return Err(SocketError::GetOption { errno: errno(), option: "SO_SOCKNAME" }.into());
		}

		unsafe {
			D::Addr::from_sockaddr(&storage as *const _ as *const libc::sockaddr, len)
				.ok_or_else(|| SocketError::InvalidAddress { reason: "invalid address" }.into())
		}
	}
}
impl<D: Domain> FromRawFd for ConnectedStream<D> {
	unsafe fn from_raw_fd(fd: RawFd) -> Self {
		unsafe { Self::from_fd(OwnedFd::from_raw_fd(fd)) }
	}
}
impl<D: Domain> IntoRawFd for ConnectedStream<D> {
	fn into_raw_fd(self) -> RawFd {
		self.fd.into_raw_fd()
	}
}

pub enum Shutdown {
	Read,   // SHUT_RD
	Write,  // SHUT_WR
	ReadWrite,   // SHUT_RDWR
}
impl<D: Domain> ConnectedStream<D> {
	pub fn shutdown(&self, how: Shutdown) -> std::io::Result<()> {
		let how = match how {
			Shutdown::Read => libc::SHUT_RD,
			Shutdown::Write => libc::SHUT_WR,
			Shutdown::ReadWrite => libc::SHUT_RDWR,
		};

		let result = unsafe { libc::shutdown(self.as_raw_fd(), how) };

		if result == -1 {
			Err(SocketError::SetOption { errno: errno(), option: "shutdown" }.into())
		} else {
			Ok(())
		}
	}
}