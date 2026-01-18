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
    /// Accepts a connection from the listening socket.
    ///
    /// Blocks until a client connects.
    /// Returns a connected stream ready for read/write.
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