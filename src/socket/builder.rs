use std::fmt::Debug;
use std::marker::PhantomData;
use crate::addr::{Domain, ToSockAddr};
use super::{
	RawSocket, Listener, ConnectedStream, BoundDatagram,
	Stream, Datagram,
	set_reuse_addr, set_reuse_port, set_tcp_nodelay,
	set_recv_buffer_size, set_send_buffer_size,
	set_keepalive, set_keepalive_idle, set_keepalive_interval, set_keepalive_count,
	set_linger,
};

// ============================================================================
// Shared Configuration Structs
// ============================================================================

/// Buffer size configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct BufferConfig {
	pub recv: Option<usize>,
	pub send: Option<usize>,
}

impl BufferConfig {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn recv(mut self, size: usize) -> Self {
		self.recv = Some(size);
		self
	}

	pub fn send(mut self, size: usize) -> Self {
		self.send = Some(size);
		self
	}

	pub fn both(mut self, size: usize) -> Self {
		self.recv = Some(size);
		self.send = Some(size);
		self
	}

	fn apply<S: std::os::fd::AsRawFd>(&self, socket: &S) -> std::io::Result<()> {
		if let Some(size) = self.recv {
			set_recv_buffer_size(socket, size)?;
		}
		if let Some(size) = self.send {
			set_send_buffer_size(socket, size)?;
		}
		Ok(())
	}
}

/// Address reuse configuration.
#[derive(Debug, Clone, Copy)]
pub struct ReuseConfig {
	pub addr: bool,
	pub port: bool,
}

impl Default for ReuseConfig {
	fn default() -> Self {
		Self {
			addr: true,  // Almost always want this for servers
			port: false,
		}
	}
}

impl ReuseConfig {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn addr(mut self, enable: bool) -> Self {
		self.addr = enable;
		self
	}

	pub fn port(mut self, enable: bool) -> Self {
		self.port = enable;
		self
	}

	/// Enable both for load balancing across threads.
	pub fn both(mut self) -> Self {
		self.addr = true;
		self.port = true;
		self
	}

	fn apply<S: std::os::fd::AsRawFd>(&self, socket: &S) -> std::io::Result<()> {
		if self.addr {
			set_reuse_addr(socket, true)?;
		}
		if self.port {
			set_reuse_port(socket, true)?;
		}
		Ok(())
	}
}

/// TCP-specific configuration.
#[derive(Debug, Clone, Copy)]
pub struct TcpConfig {
	pub nodelay: bool,
	pub keepalive: Option<KeepaliveConfig>,
	pub linger: Option<Option<u32>>,
}

impl Default for TcpConfig {
	fn default() -> Self {
		Self {
			nodelay: true,  // Low latency by default
			keepalive: None,
			linger: None,
		}
	}
}

impl TcpConfig {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn nodelay(mut self, enable: bool) -> Self {
		self.nodelay = enable;
		self
	}

	pub fn keepalive(mut self, config: KeepaliveConfig) -> Self {
		self.keepalive = Some(config);
		self
	}

	pub fn linger(mut self, seconds: Option<u32>) -> Self {
		self.linger = Some(seconds);
		self
	}

	fn apply<S: std::os::fd::AsRawFd>(&self, socket: &S, is_unix: bool) -> std::io::Result<()> {
		if !is_unix && self.nodelay {
			set_tcp_nodelay(socket, true)?;
		}
		if !is_unix {
			if let Some(config) = self.keepalive {
				set_keepalive(socket, true)?;
				set_keepalive_idle(socket, config.idle_secs)?;
				set_keepalive_interval(socket, config.interval_secs)?;
				set_keepalive_count(socket, config.count)?;
			}
		}
		if let Some(linger) = self.linger {
			set_linger(socket, linger)?;
		}
		Ok(())
	}
}

/// Keep-alive timing configuration.
#[derive(Debug, Clone, Copy)]
pub struct KeepaliveConfig {
	pub idle_secs: u32,
	pub interval_secs: u32,
	pub count: u32,
}

impl Default for KeepaliveConfig {
	fn default() -> Self {
		Self {
			idle_secs: 60,
			interval_secs: 10,
			count: 5,
		}
	}
}

impl KeepaliveConfig {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn idle(mut self, secs: u32) -> Self {
		self.idle_secs = secs;
		self
	}

	pub fn interval(mut self, secs: u32) -> Self {
		self.interval_secs = secs;
		self
	}

	pub fn count(mut self, count: u32) -> Self {
		self.count = count;
		self
	}
}

// ============================================================================
// Listener Builder
// ============================================================================

/// Builder for TCP/Unix stream listeners.
///
/// # Example
/// ```ignore
/// use wirelane::{Ipv4, SocketAddrV4, ListenerBuilder, ReuseConfig, TcpConfig, KeepaliveConfig};
///
/// let listener = ListenerBuilder::<Ipv4>::new()
///     .reuse(ReuseConfig::new().both())
///     .tcp(TcpConfig::new()
///         .nodelay(true)
///         .keepalive(KeepaliveConfig::new().idle(60).interval(10).count(5)))
///     .backlog(4096)
///     .bind(SocketAddrV4::new([0, 0, 0, 0], 8080))?;
/// ```
pub struct ListenerBuilder<D: Domain> {
	reuse: ReuseConfig,
	tcp: TcpConfig,
	buffers: BufferConfig,
	backlog: i32,
	nonblocking: bool,
	_marker: PhantomData<D>,
}

impl<D: Domain> Default for ListenerBuilder<D> {
	fn default() -> Self {
		Self::new()
	}
}

impl<D: Domain> ListenerBuilder<D> {
	pub fn new() -> Self {
		Self {
			reuse: ReuseConfig::default(),
			tcp: TcpConfig::default(),
			buffers: BufferConfig::default(),
			backlog: 128,
			nonblocking: false,
			_marker: PhantomData,
		}
	}

	/// Set address reuse options.
	pub fn reuse(mut self, config: ReuseConfig) -> Self {
		self.reuse = config;
		self
	}

	/// Set TCP options (ignored for Unix sockets).
	pub fn tcp(mut self, config: TcpConfig) -> Self {
		self.tcp = config;
		self
	}

	/// Set buffer sizes.
	pub fn buffers(mut self, config: BufferConfig) -> Self {
		self.buffers = config;
		self
	}

	/// Set listen backlog. Default: 128.
	pub fn backlog(mut self, backlog: i32) -> Self {
		self.backlog = backlog;
		self
	}

	/// Set non-blocking mode.
	pub fn nonblocking(mut self, enable: bool) -> Self {
		self.nonblocking = enable;
		self
	}

	// Legacy methods for backwards compatibility
	pub fn reuse_addr(mut self, enable: bool) -> Self {
		self.reuse.addr = enable;
		self
	}

	pub fn reuse_port(mut self, enable: bool) -> Self {
		self.reuse.port = enable;
		self
	}

	pub fn tcp_nodelay(mut self, enable: bool) -> Self {
		self.tcp.nodelay = enable;
		self
	}

	/// Binds and starts listening.
	pub fn bind(self, addr: D::Addr) -> std::io::Result<Listener<D>>
	where
		D::Addr: ToSockAddr, <D as Domain>::Addr: Debug
	{
		let socket = RawSocket::<D, Stream>::new()?;
		let is_unix = D::raw() == libc::AF_UNIX;

		self.reuse.apply(&socket)?;
		self.tcp.apply(&socket, is_unix)?;
		self.buffers.apply(&socket)?;

		if self.nonblocking {
			socket.set_nonblocking(true)?;
		}

		let bound = socket.bind(addr)?;
		bound.listen(self.backlog)
	}
}

// ============================================================================
// Connector Builder
// ============================================================================

/// Builder for TCP/Unix stream connections.
///
/// # Example
/// ```ignore
/// use wirelane::{Ipv4, SocketAddrV4, ConnectorBuilder, TcpConfig, BufferConfig};
///
/// let conn = ConnectorBuilder::<Ipv4>::new()
///     .tcp(TcpConfig::new().nodelay(true).linger(Some(5)))
///     .buffers(BufferConfig::new().both(65536))
///     .connect(SocketAddrV4::new([127, 0, 0, 1], 8080))?;
/// ```
pub struct ConnectorBuilder<D: Domain> {
	tcp: TcpConfig,
	buffers: BufferConfig,
	_marker: PhantomData<D>,
}

impl<D: Domain> Default for ConnectorBuilder<D> {
	fn default() -> Self {
		Self::new()
	}
}

impl<D: Domain> ConnectorBuilder<D> {
	pub fn new() -> Self {
		Self {
			tcp: TcpConfig::default(),
			buffers: BufferConfig::default(),
			_marker: PhantomData,
		}
	}

	/// Set TCP options.
	pub fn tcp(mut self, config: TcpConfig) -> Self {
		self.tcp = config;
		self
	}

	/// Set buffer sizes.
	pub fn buffers(mut self, config: BufferConfig) -> Self {
		self.buffers = config;
		self
	}

	// Legacy method
	pub fn tcp_nodelay(mut self, enable: bool) -> Self {
		self.tcp.nodelay = enable;
		self
	}

	/// Connects to the remote address.
	pub fn connect(self, addr: D::Addr) -> std::io::Result<ConnectedStream<D>>
	where
		D::Addr: ToSockAddr, <D as Domain>::Addr: Debug
	{
		let socket = RawSocket::<D, Stream>::new()?;
		let is_unix = D::raw() == libc::AF_UNIX;

		self.tcp.apply(&socket, is_unix)?;
		self.buffers.apply(&socket)?;

		socket.connect(addr)
	}
}

// ============================================================================
// Datagram Builder
// ============================================================================

/// Builder for UDP/Unix datagram sockets.
///
/// # Example
/// ```ignore
/// use wirelane::{Ipv4, SocketAddrV4, DatagramBuilder, ReuseConfig, BufferConfig};
///
/// let socket = DatagramBuilder::<Ipv4>::new()
///     .reuse(ReuseConfig::new().addr(true))
///     .buffers(BufferConfig::new().recv(1048576))  // 1MB receive buffer
///     .bind(SocketAddrV4::new([0, 0, 0, 0], 5353))?;
/// ```
pub struct DatagramBuilder<D: Domain> {
	reuse: ReuseConfig,
	buffers: BufferConfig,
	_marker: PhantomData<D>,
}

impl<D: Domain> Default for DatagramBuilder<D> {
	fn default() -> Self {
		Self::new()
	}
}

impl<D: Domain> DatagramBuilder<D> {
	pub fn new() -> Self {
		Self {
			reuse: ReuseConfig { addr: false, port: false },
			buffers: BufferConfig::default(),
			_marker: PhantomData,
		}
	}

	/// Set address reuse options.
	pub fn reuse(mut self, config: ReuseConfig) -> Self {
		self.reuse = config;
		self
	}

	/// Set buffer sizes.
	pub fn buffers(mut self, config: BufferConfig) -> Self {
		self.buffers = config;
		self
	}

	// Legacy methods
	pub fn reuse_addr(mut self, enable: bool) -> Self {
		self.reuse.addr = enable;
		self
	}

	pub fn reuse_port(mut self, enable: bool) -> Self {
		self.reuse.port = enable;
		self
	}

	/// Binds to an address.
	pub fn bind(self, addr: D::Addr) -> std::io::Result<BoundDatagram<D>>
	where
		D::Addr: ToSockAddr, <D as Domain>::Addr: Debug
	{
		let socket = RawSocket::<D, Datagram>::new()?;

		self.reuse.apply(&socket)?;
		self.buffers.apply(&socket)?;

		socket.bind_datagram(addr)
	}
}