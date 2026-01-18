mod listener;
mod raw;
mod stream;
mod datagram;
mod options;
mod bound;
mod builder;
mod pending;

pub use self::listener::Listener;
pub use self::raw::RawSocket;
pub use self::stream::{ConnectedStream,Shutdown};
pub use self::bound::BoundSocket;
pub use self::datagram::{BoundDatagram, ConnectedDatagram, SendMsg, MsgResult};
pub use self::options::{set_reuse_addr, set_reuse_port, set_tcp_nodelay, set_linger,
						set_recv_buffer_size, set_keepalive, set_keepalive_count,
						set_keepalive_idle, set_keepalive_interval, set_send_buffer_size,
						set_tcp_cork, set_tcp_quickack, set_tcp_fastopen,
						get_tcp_info, TcpInfo,
						splice, SPLICE_F_MOVE, SPLICE_F_NONBLOCK, SPLICE_F_MORE,
						send_fd, recv_fd};
pub use self::pending::PendingConnect;


pub use self::builder::{ListenerBuilder, ConnectorBuilder, DatagramBuilder,
						BufferConfig, ReuseConfig, TcpConfig, KeepaliveConfig};

/// Trait for socket type markers.
///
/// Each type implementing this trait represents a socket type
/// that can be passed to the `socket()` syscall.
///
/// - `Stream` — reliable, ordered byte stream (TCP-like)
/// - `Datagram` — unreliable, unordered packets (UDP-like)
pub trait SockType {
	/// Returns the libc constant for this socket type.
	fn raw() -> libc::c_int;
}


/// Stream socket marker.
///
/// Provides reliable, ordered, two-way byte streams.
/// Used for TCP (with Ipv4/Ipv6) or Unix stream sockets.
pub struct Stream;

/// Datagram socket marker.
///
/// Provides unreliable, unordered packets.
/// Used for UDP (with Ipv4/Ipv6) or Unix datagram sockets.
pub struct Datagram;

/*
 ---
  Key difference:
  ┌──────────┬────────────────────────────────────────┬──────────────────────┐
  │   Type   │               Guarantees               │       Use case       │
  ├──────────┼────────────────────────────────────────┼──────────────────────┤
  │ Stream   │ Ordered, reliable, no boundaries       │ TCP, HTTP, databases │
  ├──────────┼────────────────────────────────────────┼──────────────────────┤
  │ Datagram │ Fast, no guarantees, packet boundaries │ DNS, gaming, video   │
  └──────────┴────────────────────────────────────────┴──────────────────────┘
  ---
*/

impl SockType for Stream {
	#[inline]
	fn raw() -> libc::c_int {
		libc::SOCK_STREAM
	}
}

impl SockType for Datagram {
	#[inline]
	fn raw() -> libc::c_int {
		libc::SOCK_DGRAM
	}
}

/*
---
  The mapping:
  ┌──────────┬─────────────┬───────────────┐
  │  Marker  │  Constant   │ Value (Linux) │
  ├──────────┼─────────────┼───────────────┤
  │ Stream   │ SOCK_STREAM │ 1             │
  ├──────────┼─────────────┼───────────────┤
  │ Datagram │ SOCK_DGRAM  │ 2             │
  └──────────┴─────────────┴───────────────┘
  ---
*/