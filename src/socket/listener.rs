use crate::addr::{Domain, FromSockAddr};
use crate::socket::{Stream, bound::BoundSocket};
use crate::error::{SocketError, errno};
use super::stream::ConnectedStream;
use std::{marker::PhantomData, os::fd::OwnedFd};


/// A listening socket ready to accept connections.
///
/// Only exists for Stream sockets — you cannot listen on datagrams.
/// The type parameter D tracks which address family (Ipv4, Ipv6, Unix).
pub struct Listener<D: Domain> {
    fd: OwnedFd,
    _marker: PhantomData<D>,
}

impl<D: Domain> Listener<D> {
    /// Creates a Listener from an OwnedFd.
    ///
    /// Internal use only — called by BoundSocket::listen()
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
    /// Accepts an incoming connection **using blocking semantics**.
    ///
    /// # Guarantees
    ///
    /// When this function returns `Ok(ConnectedStream<D>)`, the kernel guarantees:
    ///
    /// - The TCP handshake has completed
    /// - The returned socket is connected
    /// - The socket is **ready for read/write**
    /// - A subsequent `read()` will block until data arrives
    ///
    /// This strong postcondition is only valid when:
    /// - The listener file descriptor is **blocking**
    /// - Or readiness has already been ensured by the caller
    ///
    /// # Failure Modes
    ///
    /// - Returns an error if the syscall fails
    /// - May return `EAGAIN` if the listener is non-blocking
    ///
    /// # Why this returns `ConnectedStream<D>`
    ///
    /// Blocking `accept()` delegates waiting to the kernel.
    /// Because the kernel blocks internally, this function can return
    /// a fully usable `ConnectedStream` without ambiguity.
    pub fn accept(&self) -> std::io::Result<ConnectedStream<D>> {
        use std::os::fd::FromRawFd;
        let fd = unsafe {
            libc::accept4(
                self.as_raw_fd(),
                std::ptr::null_mut(),    // We don't need client address
                std::ptr::null_mut(),    // No address length
                libc::SOCK_CLOEXEC,      // Close on exec
            )
        };

        if fd == -1 {
            return Err(SocketError::Accept { errno: errno() }.into());
        }

        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        Ok(ConnectedStream::from_fd(fd))
    }
    
    /// Sets or clears the `O_NONBLOCK` flag on the listener socket.
    ///
    /// # Important
    ///
    /// This affects:
    /// - Whether `accept()` blocks
    /// - Whether `accept_nonblocking()` returns `WouldBlock`
    ///
    /// This does **not** change typestate.
    /// Blocking behavior is a runtime property, not a state transition.
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
impl<D: Domain> Listener<D>
where
    D::Addr: FromSockAddr,
{
    /// Accepts a connection, returning the client's address.
    ///
    /// Use this when you need to know who connected (logging, rate limiting, etc.).
    pub fn accept_with_addr(&self) -> std::io::Result<(ConnectedStream<D>, D::Addr)> {
        use std::os::fd::FromRawFd;
        let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

        let fd = unsafe {
            libc::accept4(
                self.as_raw_fd(),
                &mut storage as *mut _ as *mut libc::sockaddr,
                &mut len,
                libc::SOCK_CLOEXEC,
            )
        };

        if fd == -1 {
            return Err(SocketError::Accept { errno: errno() }.into());
        }

        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let stream = ConnectedStream::from_fd(fd);

        let addr = unsafe {
            D::Addr::from_sockaddr(&storage as *const _ as *const libc::sockaddr, len)
                .ok_or_else(|| SocketError::InvalidAddress {
                    reason: "invalid client address",
                })?
        };

        Ok((stream, addr))
    }
    
    
    /// Attempts to accept a connection **without blocking**.
    ///
    /// # Semantics
    ///
    /// This method **never blocks**.
    /// It reports the kernel's current state explicitly.
    ///
    /// Possible outcomes:
    ///
    /// - `AcceptResult::Connection`
    ///   - A connection was accepted
    ///   - The returned socket is connected
    ///   - **Read/write readiness is NOT guaranteed**
    ///
    /// - `AcceptResult::WouldBlock`
    ///   - No pending connections exist
    ///   - Caller must wait for readiness (epoll / io_uring / retry)
    ///
    /// - `AcceptResult::Interrupted`
    ///   - The syscall was interrupted by a signal
    ///   - Safe to retry immediately
    ///
    /// # Critical Difference from `accept()`
    ///
    /// A successful non-blocking accept **does NOT guarantee**
    /// that data is available to read.
    ///
    /// Calling `read()` immediately may return `EAGAIN`.
    ///
    /// # Correct Usage
    ///
    /// After receiving `Connection`:
    ///
    /// - Wait for read readiness **or**
    /// - Retry `read()` on `WouldBlock`
    ///
    /// # Why this does NOT return `ConnectedStream<D>` directly
    ///
    /// Non-blocking accept has a weaker postcondition.
    /// Returning `ConnectedStream<D>` alone would incorrectly imply
    /// read/write readiness.
    pub fn accept_nonblocking(&self) -> std::io::Result<AcceptResult<D>> {
        use std::os::fd::FromRawFd;
        
        let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
        
        let fd = unsafe {
            libc::accept4(
                self.as_raw_fd(),
                &mut storage as *mut _ as *mut libc::sockaddr,
                &mut len,
                libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            )
        };
        
        if fd == -1 {
            let err = errno();
            return match err {
                libc::EAGAIN => Ok(AcceptResult::WouldBlock),
                libc::EINTR => Ok(AcceptResult::Interrupted),
                _ => Err(SocketError::Accept { errno: err }.into()),
            };
        }
        
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let stream = ConnectedStream::from_fd(fd);
        
        let addr = unsafe {
            D::Addr::from_sockaddr(&storage as *const _ as *const libc::sockaddr, len)
                .ok_or_else(|| SocketError::InvalidAddress {
                    reason: "invalid client address",
                })?
        };
        
        Ok(AcceptResult::Connection(stream, addr))
    }
}

impl<D: Domain> std::os::fd::AsRawFd for Listener<D> {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.fd.as_raw_fd()
    }
}

impl<D: Domain> std::os::fd::AsFd for Listener<D> {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.fd.as_fd()
    }
}
impl<D: Domain> std::os::fd::FromRawFd for Listener<D> {
    unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
        unsafe { Self::from_fd(OwnedFd::from_raw_fd(fd)) }
    }
}

impl<D: Domain> std::os::fd::IntoRawFd for Listener<D> {
    fn into_raw_fd(self) -> std::os::fd::RawFd {
        self.fd.into_raw_fd()
    }
}

/*
Notice: Listener<D> has no T: SockType parameter.
Why? Because by the time you're a Listener, you are definitely a Stream socket.
The type T = Stream is already known — it's baked into the fact that you're a Listener at all.
This is the typestate pattern working: the type itself carries the information.
*/


impl<D: Domain> BoundSocket<D, Stream> {
    /// Transitions to a listening socket.
    ///
    /// `backlog` — maximum pending connections queue size.
    /// Typical values: 128 for small services, 4096+ for high-traffic servers.
    ///
    /// Consumes self — you cannot use BoundSocket after this.
    /// Returns Listener<D> ready for accept().
    pub fn listen(self, backlog: i32) -> std::io::Result<Listener<D>> {
        let result = unsafe {
            libc::listen(self.as_raw_fd(), backlog)
        };

        if result == -1 {
            return Err(SocketError::Listen { errno: errno(), backlog }.into());
        }

        // Extract the fd from self without running Drop
        let fd = self.into_fd();

        Ok(Listener::from_fd(fd))
    }
}
/*
The signature impl<D: Domain> BoundSocket<D, Stream>
means: "this impl block only applies when T = Stream". 
If someone has a BoundSocket<Ipv4, Datagram>,they cannot call .listen()
 — the compiler will say "method not found".
*/
/// Result of a non-blocking accept attempt.
///
/// This enum does **not** represent socket state.
/// It represents the **outcome of a syscall probe**.
///
/// The listener remains in the `Listener<D>` state in all cases.
pub enum AcceptResult<D: Domain>
where
    D::Addr: FromSockAddr,
{
    /// A connection was accepted.
    ///
    /// The socket is connected but **may not yet be readable**.
    /// Callers must handle `read()` returning `WouldBlock`.
    Connection(ConnectedStream<D>, D::Addr),
    
    /// No connection is ready at this time.
    ///
    /// This is not an error.
    /// The caller must wait for readiness or retry.
    WouldBlock,
    
    /// The accept syscall was interrupted by a signal.
    ///
    /// Safe to retry immediately.
    Interrupted,
}