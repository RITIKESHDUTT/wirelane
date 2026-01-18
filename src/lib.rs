pub mod socket;
mod addr;
mod error;

pub use self::error::{IoError, SocketError, errno};
pub use self::addr::{Domain, Ipv4, Ipv6, Unix, SocketAddrV4, SocketAddrV6, UnixAddr};
pub use self::socket::{MsgResult,Shutdown, SockType, Stream, ListenerBuilder, ConnectorBuilder,
					   set_recv_buffer_size,
					   DatagramBuilder, BufferConfig, ReuseConfig, TcpConfig, KeepaliveConfig,
					   Datagram, RawSocket, BoundSocket,
					   ConnectedDatagram, Listener, ConnectedStream, BoundDatagram,
					   PendingConnect};
pub use self::socket::{set_reuse_addr, set_reuse_port, set_tcp_nodelay,
					   set_tcp_cork, set_tcp_quickack, set_tcp_fastopen,
					   get_tcp_info, TcpInfo,
					   set_keepalive, set_keepalive_idle, set_keepalive_interval, set_keepalive_count,
					   set_linger, set_send_buffer_size,
					   splice, SPLICE_F_MOVE, SPLICE_F_NONBLOCK, SPLICE_F_MORE,
					   send_fd, recv_fd, SendMsg};