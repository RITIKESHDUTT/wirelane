use std::os::fd::OwnedFd;
use std::marker::PhantomData;
use crate::addr::Domain;
use super::SockType;

/// A socket that has been bound to an address but not yet listening.
///
/// Same structure as RawSocket. Different name = different capabilities
/// For Stream sockets: call `.listen()` to become a Listener.
/// For Datagram sockets: you typically don't reach this state
/// (bind returns Datagram directly).
pub struct BoundSocket<D: Domain, T: SockType> {
	fd: OwnedFd,
	_marker: PhantomData<(D, T)>,
}


impl<D: Domain, T: SockType> BoundSocket<D, T> {
	/// Creates a BoundSocket from an OwnedFd.
	///
	/// Internal use only - called by RawSocket::bind()
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
	/// Extracts the owned file descriptor, consuming self.
	pub(crate) fn into_fd(self) -> OwnedFd {
		self.fd
	}
	
}
impl<D: Domain, T: SockType> std::os::fd::AsRawFd for BoundSocket<D, T> {
	fn as_raw_fd(&self) -> std::os::fd::RawFd {
		self.fd.as_raw_fd()
	}
}

impl<D: Domain, T: SockType> std::os::fd::AsFd for BoundSocket<D, T> {
	fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
		self.fd.as_fd()
	}
}

impl<D: Domain, T: SockType> std::os::fd::FromRawFd for BoundSocket<D, T> {
	unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
		unsafe { Self::from_fd(OwnedFd::from_raw_fd(fd)) }
	}
}

impl<D: Domain, T: SockType> std::os::fd::IntoRawFd for BoundSocket<D, T> {
	fn into_raw_fd(self) -> std::os::fd::RawFd {
		self.fd.into_raw_fd()
	}
}