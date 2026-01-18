/// Socket creation/configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum SocketError {
    #[error("socket() failed: {}", errno_to_str(*.errno))]
    Create { errno: i32 },
    
    #[error("bind({addr}) failed: {}", errno_to_str(*.errno))]
    Bind { errno: i32, addr: String },
    
    #[error("listen(backlog={backlog}) failed: {}", errno_to_str(*.errno))]
    Listen { errno: i32, backlog: i32 },
    
    #[error("connect({addr}) failed: {}", errno_to_str(*.errno))]
    Connect { errno: i32, addr: String },
    
    #[error("accept() failed: {}", errno_to_str(*.errno))]
    Accept { errno: i32 },
    
    #[error("setsockopt({option}) failed: {}", errno_to_str(*.errno))]
    SetOption { errno: i32, option: &'static str },
    
    #[error("getsockopt({option}) failed: {}", errno_to_str(*.errno))]
    GetOption { errno: i32, option: &'static str },
    
    #[error("invalid address: {reason}")]
    InvalidAddress { reason: &'static str },
}

/// I/O operation errors.
#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("read() failed: {}", errno_to_str(*.errno))]
    Read { errno: i32 },
    
    #[error("write() failed: {}", errno_to_str(*.errno))]
    Write { errno: i32 },
    
    #[error("connection closed by peer")]
    ConnectionClosed,
    
    #[error("operation would block")]
    WouldBlock,
    
    #[error("interrupted by signal")]
    Interrupted,
}

/// Returns current errno value.
#[inline]
pub fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

/// Converts errno to human-readable string.
fn errno_to_str(errno: i32) -> String {
    match errno {
        libc::EACCES => "permission denied".into(),
        libc::EADDRINUSE => "address already in use".into(),
        libc::EADDRNOTAVAIL => "address not available".into(),
        libc::EAFNOSUPPORT => "address family not supported".into(),
        libc::EAGAIN => "resource temporarily unavailable".into(),
        libc::EBADF => "bad file descriptor".into(),
        libc::ECONNREFUSED => "connection refused".into(),
        libc::ECONNRESET => "connection reset by peer".into(),
        libc::EINPROGRESS => "operation in progress".into(),
        libc::EINTR => "interrupted by signal".into(),
        libc::EINVAL => "invalid argument".into(),
        libc::EMFILE => "too many open files".into(),
        libc::ENETUNREACH => "network unreachable".into(),
        libc::ENOBUFS => "no buffer space available".into(),
        libc::ENOTCONN => "not connected".into(),
        libc::EPIPE => "broken pipe".into(),
        libc::ETIMEDOUT => "connection timed out".into(),
        _ => format!("errno {}", errno),
    }
}

/// Maps errno to std::io::ErrorKind.
fn errno_to_kind(errno: i32) -> std::io::ErrorKind {
    match errno {
        libc::EACCES | libc::EPERM => std::io::ErrorKind::PermissionDenied,
        libc::EADDRINUSE => std::io::ErrorKind::AddrInUse,
        libc::EADDRNOTAVAIL => std::io::ErrorKind::AddrNotAvailable,
        libc::EAGAIN | libc::EWOULDBLOCK => std::io::ErrorKind::WouldBlock,
        libc::ECONNREFUSED => std::io::ErrorKind::ConnectionRefused,
        libc::ECONNRESET => std::io::ErrorKind::ConnectionReset,
        libc::EINTR => std::io::ErrorKind::Interrupted,
        libc::EINVAL => std::io::ErrorKind::InvalidInput,
        libc::ENOTCONN => std::io::ErrorKind::NotConnected,
        libc::EPIPE => std::io::ErrorKind::BrokenPipe,
        libc::ETIMEDOUT => std::io::ErrorKind::TimedOut,
        _ => std::io::ErrorKind::Other,
    }
}

impl From<SocketError> for std::io::Error {
    fn from(err: SocketError) -> Self {
        let errno = match &err {
            SocketError::Create { errno } => *errno,
            SocketError::Bind { errno, .. } => *errno,
            SocketError::Listen { errno, .. } => *errno,
            SocketError::Connect { errno, .. } => *errno,
            SocketError::Accept { errno } => *errno,
            SocketError::SetOption { errno, .. } => *errno,
            SocketError::GetOption { errno, .. } => *errno,
            SocketError::InvalidAddress { .. } => libc::EINVAL,
        };
        std::io::Error::new(errno_to_kind(errno), err)
    }
}

impl From<IoError> for std::io::Error {
    fn from(err: IoError) -> Self {
        let kind = match &err {
            IoError::Read { errno } => errno_to_kind(*errno),
            IoError::Write { errno } => errno_to_kind(*errno),
            IoError::ConnectionClosed => std::io::ErrorKind::ConnectionReset,
            IoError::WouldBlock => std::io::ErrorKind::WouldBlock,
            IoError::Interrupted => std::io::ErrorKind::Interrupted,
        };
        std::io::Error::new(kind, err)
    }
}