use crate::Domain;
use std::marker::PhantomData;
use std::os::fd::OwnedFd;
use super::stream::ConnectedStream;

pub struct PendingConnect<D: Domain> {
      fd: OwnedFd,
      _marker: PhantomData<D>,
}
impl<D: Domain> PendingConnect<D> {
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
    /// Reads and clears the socket error status.
    ///
    /// Returns `None` if no error (connect succeeded).
    /// Returns `Some(error)` if connect failed.
    ///
    /// Call this after epoll/io_uring signals the socket is writable.
    /// Reading clears the error — only call once.
    pub fn take_error(&self) -> std::io::Result<Option<std::io::Error>> {
        let mut error: libc::c_int = 0;
        let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
        
        let result = unsafe {
            libc::getsockopt(
                self.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                &mut error as *mut _ as *mut libc::c_void,
                &mut len,
            )
        };
        
        if result == -1 {
            return Err(std::io::Error::last_os_error());
        }
        
        if error == 0 {
            Ok(None)  // Success, no error
        } else {
            Ok(Some(std::io::Error::from_raw_os_error(error)))
        }
    }
    /// Completes the connection after verifying no error.
    ///
    /// Call `take_error()` first. If it returned `None`, call this.
    /// Consumes self, returns the connected stream.
    pub fn finish(self) -> ConnectedStream<D> {
        ConnectedStream::from_fd(self.fd)
    }
    
}
/*
 Breaking it down:
  ┌───────────────────────────────────────────┬───────────────────────────────────────────┐
  │                   Line                    │                  Purpose                  │
  ├───────────────────────────────────────────┼───────────────────────────────────────────┤
  │ let mut error: libc::c_int = 0            │ Kernel writes the error code here         │
  ├───────────────────────────────────────────┼───────────────────────────────────────────┤
  │ libc::SOL_SOCKET                          │ Socket-level option (not TCP/IP specific) │
  ├───────────────────────────────────────────┼───────────────────────────────────────────┤
  │ libc::SO_ERROR                            │ The specific option we want               │
  ├───────────────────────────────────────────┼───────────────────────────────────────────┤
  │ &mut error as *mut _ as *mut libc::c_void │ Kernel needs void pointer                 │
  ├───────────────────────────────────────────┼───────────────────────────────────────────┤
  │ error == 0                                │ Zero means no error, connect succeeded    │
  ├───────────────────────────────────────────┼───────────────────────────────────────────┤
  │ from_raw_os_error(error)                  │ Convert errno to Rust Error               │
  └───────────────────────────────────────────┴───────────────────────────────────────────
 */

impl<D: Domain> std::os::fd::AsRawFd for PendingConnect<D> {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.fd.as_raw_fd()
    }
}

impl<D: Domain> std::os::fd::AsFd for PendingConnect<D> {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl<D: Domain> std::os::fd::FromRawFd for PendingConnect<D> {
    unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
        unsafe { Self::from_fd(OwnedFd::from_raw_fd(fd)) }
    }
}

impl<D: Domain> std::os::fd::IntoRawFd for PendingConnect<D> {
    fn into_raw_fd(self) -> std::os::fd::RawFd {
        self.fd.into_raw_fd()
    }
}