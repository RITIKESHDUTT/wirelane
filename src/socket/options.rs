use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::os::fd::RawFd;
use std::os::fd::AsRawFd;
use crate::error::{SocketError, IoError, errno};

/// Sets SO_REUSEADDR on a socket.
///
/// Allows binding to an address that's in TIME_WAIT state.
/// Essential for server restarts.
pub fn set_reuse_addr<S: AsRawFd>(socket: &S, enable: bool) -> std::io::Result<()> {
	let val: libc::c_int = if enable { 1 } else { 0 };
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::SOL_SOCKET,
			libc::SO_REUSEADDR,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// Sets SO_REUSEPORT on a socket.
///
/// Allows multiple sockets to bind the same port.
/// Used for load balancing across threads/processes.
pub fn set_reuse_port<S: AsRawFd>(socket: &S, enable: bool) -> std::io::Result<()> {
	let val: libc::c_int = if enable { 1 } else { 0 };
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::SOL_SOCKET,
			libc::SO_REUSEPORT,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// Sets TCP_NODELAY on a socket.
///
/// Disables Nagle's algorithm — sends data immediately.
/// Use for low-latency protocols (games, trading).
pub fn set_tcp_nodelay<S: AsRawFd>(socket: &S, enable: bool) -> std::io::Result<()> {
	let val: libc::c_int = if enable { 1 } else { 0 };
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::IPPROTO_TCP,
			libc::TCP_NODELAY,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// Sets receive buffer size (SO_RCVBUF).
///
/// Controls how much data the kernel buffers for incoming packets.
/// Kernel typically doubles this value internally.
/// Larger buffers help absorb traffic bursts but use more memory.
///
/// For 2M connections: be conservative (e.g., 8KB-32KB per socket).
/// 2M × 32KB = 64GB just for receive buffers.
pub fn set_recv_buffer_size<S: AsRawFd>(socket: &S, size: usize) -> std::io::Result<()> {
	let val = size as libc::c_int;
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::SOL_SOCKET,
			libc::SO_RCVBUF,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// Sets send buffer size (SO_SNDBUF).
///
/// Controls how much outgoing data the kernel buffers before blocking/returning EAGAIN.
/// Kernel typically doubles this value internally.
/// Larger buffers allow more in-flight data but use more memory.
///
/// For 2M connections: be conservative. Same math as recv buffers.
pub fn set_send_buffer_size<S: AsRawFd>(socket: &S, size: usize) -> std::io::Result<()> {
	let val = size as libc::c_int;
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::SOL_SOCKET,
			libc::SO_SNDBUF,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}


/// Enables TCP keep-alive (SO_KEEPALIVE).
///
/// When enabled, the kernel sends probes on idle connections to detect dead peers.
/// Essential for long-lived connections and detecting half-open sockets.
/// Use with TCP_KEEPIDLE, TCP_KEEPINTVL, TCP_KEEPCNT to tune timing.
pub fn set_keepalive<S: AsRawFd>(socket: &S, enable: bool) -> std::io::Result<()> {
	let val: libc::c_int = if enable { 1 } else { 0 };
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::SOL_SOCKET,
			libc::SO_KEEPALIVE,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// Sets TCP keep-alive idle time (TCP_KEEPIDLE).
///
/// Seconds of idle time before the first keep-alive probe is sent.
/// Default is typically 7200 (2 hours). For servers, 60-300 is common.
/// Requires SO_KEEPALIVE to be enabled.
pub fn set_keepalive_idle<S: AsRawFd>(socket: &S, seconds: u32) -> std::io::Result<()> {
	let val = seconds as libc::c_int;
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::IPPROTO_TCP,
			libc::TCP_KEEPIDLE,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// Sets TCP keep-alive probe interval (TCP_KEEPINTVL).
///
/// Seconds between successive keep-alive probes if no response.
/// Default is typically 75. For faster dead-peer detection, use 10-30.
/// Requires SO_KEEPALIVE to be enabled.
pub fn set_keepalive_interval<S: AsRawFd>(socket: &S, seconds: u32) -> std::io::Result<()> {
	let val = seconds as libc::c_int;
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::IPPROTO_TCP,
			libc::TCP_KEEPINTVL,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// Sets TCP keep-alive probe count (TCP_KEEPCNT).
///
/// Number of unacknowledged probes before connection is considered dead.
/// Default is typically 9. For faster detection, use 3-5.
/// Total detection time = KEEPIDLE + (KEEPINTVL × KEEPCNT).
/// Requires SO_KEEPALIVE to be enabled.
pub fn set_keepalive_count<S: AsRawFd>(socket: &S, count: u32) -> std::io::Result<()> {
	let val = count as libc::c_int;
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::IPPROTO_TCP,
			libc::TCP_KEEPCNT,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}
/// Sets socket linger behavior (SO_LINGER).
///
/// Controls what happens when close() is called with unsent data:
/// - `None` — default behavior, close returns immediately, kernel sends data in background
/// - `Some(0)` — hard reset (RST), discards unsent data, no TIME_WAIT
/// - `Some(n)` — close blocks up to n seconds waiting for data to send
///
/// For high-connection servers: `Some(0)` avoids TIME_WAIT buildup but is aggressive.
/// For graceful shutdown: `None` or `Some(30)`.
pub fn set_linger<S: AsRawFd>(socket: &S, linger: Option<u32>) -> std::io::Result<()> {
	let val = match linger {
		None => libc::linger { l_onoff: 0, l_linger: 0 },
		Some(seconds) => libc::linger {
			l_onoff: 1,
			l_linger: seconds as libc::c_int
		},
	};
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::SOL_SOCKET,
			libc::SO_LINGER,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::linger>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}
pub const SPLICE_F_MOVE: u32 = libc::SPLICE_F_MOVE as u32;
pub const SPLICE_F_NONBLOCK: u32 = libc::SPLICE_F_NONBLOCK as u32;
pub const SPLICE_F_MORE: u32 = libc::SPLICE_F_MORE as u32;

/// Moves data between two file descriptors without copying to userspace.
///
/// One of `fd_in` or `fd_out` must be a pipe.
/// Returns number of bytes transferred.
///
/// Flags:
/// - `SPLICE_F_MOVE`: hint to move pages (not always honored)
/// - `SPLICE_F_NONBLOCK`: non-blocking
/// - `SPLICE_F_MORE`: more data coming (like TCP_CORK)
pub fn splice<In: AsRawFd, Out: AsRawFd>(
	fd_in: &In,
	off_in: Option<&mut i64>,
	fd_out: &Out,
	off_out: Option<&mut i64>,
	len: usize,
	flags: u32,
) -> std::io::Result<usize> {
	let off_in_ptr = off_in.map_or(std::ptr::null_mut(), |o| o as *mut i64);
	let off_out_ptr = off_out.map_or(std::ptr::null_mut(), |o| o as *mut i64);
	
	let n = unsafe {
		libc::splice(
			fd_in.as_raw_fd(),
			off_in_ptr,
			fd_out.as_raw_fd(),
			off_out_ptr,
			len,
			flags as libc::c_uint,
		)
	};

	if n == -1 {
		Err(IoError::Write { errno: errno() }.into())
	} else {
		Ok(n as usize)
	}
}

/// Sets TCP_CORK — buffers small writes until uncorked.
///
/// When enabled, TCP holds data until:
/// - Cork is disabled
/// - Buffer is full
/// - 200ms timeout
///
/// Use for batching HTTP headers + body into one packet.
pub fn set_tcp_cork<S: AsRawFd>(socket: &S, enable: bool) -> std::io::Result<()> {
	let val: libc::c_int = if enable { 1 } else { 0 };
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::IPPROTO_TCP,
			libc::TCP_CORK,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// Sets TCP_QUICKACK — disables delayed ACKs.
///
/// Normally TCP waits ~40ms hoping to piggyback ACK on response data.
/// This forces immediate ACK after receiving data.
///
/// Note: Kernel may reset this after each received packet.
/// Re-apply after each recv() if needed.
pub fn set_tcp_quickack<S: AsRawFd>(socket: &S, enable: bool) -> std::io::Result<()> {
	let val: libc::c_int = if enable { 1 } else { 0 };
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::IPPROTO_TCP,
			libc::TCP_QUICKACK,
			&val as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// Enables TCP Fast Open on a listening socket.
///
/// `queue_len` is max pending TFO connections (typically 5-10).
/// Allows clients to send data in the SYN packet.
pub fn set_tcp_fastopen<S: AsRawFd>(socket: &S, queue_len: i32) -> std::io::Result<()> {
	let result = unsafe {
		libc::setsockopt(
			socket.as_raw_fd(),
			libc::IPPROTO_TCP,
			libc::TCP_FASTOPEN,
			&queue_len as *const _ as *const libc::c_void,
			std::mem::size_of::<libc::c_int>() as libc::socklen_t,
		)
	};
	if result == -1 {
		Err(SocketError::SetOption { errno: errno(), option: "socket option" }.into())
	} else {
		Ok(())
	}
}

/// TCP connection statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct TcpInfo {
	pub state: u8,
	pub retransmits: u8,
	pub probes: u8,
	pub backoff: u8,
	pub rtt_us: u32,          // Round-trip time in microseconds
	pub rtt_var_us: u32,      // RTT variance
	pub snd_cwnd: u32,        // Congestion window (packets)
	pub rcv_rtt_us: u32,      // Receiver RTT
	pub total_retrans: u32,   // Total retransmissions
}

/// Gets TCP connection statistics.
///
/// Returns RTT, congestion window, retransmit count, etc.
/// Useful for monitoring and adaptive protocols.
pub fn get_tcp_info<S: AsRawFd>(socket: &S) -> std::io::Result<TcpInfo> {
	let mut info: libc::tcp_info = unsafe { std::mem::zeroed() };
	let mut len = std::mem::size_of::<libc::tcp_info>() as libc::socklen_t;
	
	let result = unsafe {
		libc::getsockopt(
			socket.as_raw_fd(),
			libc::IPPROTO_TCP,
			libc::TCP_INFO,
			&mut info as *mut _ as *mut libc::c_void,
			&mut len,
		)
	};
	
	if result == -1 {
		return Err(SocketError::GetOption { errno: errno(), option: "TCP_INFO" }.into());
	}

	Ok(TcpInfo {
		state: info.tcpi_state,
		retransmits: info.tcpi_retransmits,
		probes: info.tcpi_probes,
		backoff: info.tcpi_backoff,
		rtt_us: info.tcpi_rtt,
		rtt_var_us: info.tcpi_rttvar,
		snd_cwnd: info.tcpi_snd_cwnd,
		rcv_rtt_us: info.tcpi_rcv_rtt,
		total_retrans: info.tcpi_total_retrans,
	})
}

/// Sends a file descriptor over a Unix socket.
///
/// The receiving process gets a new fd pointing to the same resource.
pub fn send_fd<S: AsRawFd, F: AsRawFd>(socket: &S, fd: &F) -> std::io::Result<()> {
	let fd_to_send = fd.as_raw_fd();
	
	// Control message buffer
	let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<RawFd>() as u32) } as usize;
	let mut cmsg_buf = vec![0u8; cmsg_space];
	
	// Dummy data (must send at least 1 byte)
	let dummy = [0u8; 1];
	let mut iov = libc::iovec {
		iov_base: dummy.as_ptr() as *mut libc::c_void,
		iov_len: 1,
	};
	
	let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
	msg.msg_iov = &mut iov;
	msg.msg_iovlen = 1;
	msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
	msg.msg_controllen = cmsg_space;
	
	// Set up control message
	let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
	unsafe {
		(*cmsg).cmsg_level = libc::SOL_SOCKET;
		(*cmsg).cmsg_type = libc::SCM_RIGHTS;
		(*cmsg).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<RawFd>() as u32) as usize;
		std::ptr::copy_nonoverlapping(
			&fd_to_send as *const RawFd,
			libc::CMSG_DATA(cmsg) as *mut RawFd,
			1,
		);
	}
	
	let result = unsafe { libc::sendmsg(socket.as_raw_fd(), &msg, 0) };

	if result == -1 {
		Err(IoError::Write { errno: errno() }.into())
	} else {
		Ok(())
	}
}

/// Receives a file descriptor from a Unix socket.
///
/// Returns the received fd as OwnedFd.
pub fn recv_fd<S: AsRawFd>(socket: &S) -> std::io::Result<OwnedFd> {
	let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<RawFd>() as u32) } as usize;
	let mut cmsg_buf = vec![0u8; cmsg_space];
	
	let mut dummy = [0u8; 1];
	let mut iov = libc::iovec {
		iov_base: dummy.as_mut_ptr() as *mut libc::c_void,
		iov_len: 1,
	};
	
	let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
	msg.msg_iov = &mut iov;
	msg.msg_iovlen = 1;
	msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
	msg.msg_controllen = cmsg_space;
	
	let result = unsafe { libc::recvmsg(socket.as_raw_fd(), &mut msg, 0) };

	if result == -1 {
		return Err(IoError::Read { errno: errno() }.into());
	}

	// Extract fd from control message
	let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
	if cmsg.is_null() {
		return Err(SocketError::InvalidAddress { reason: "no control message received" }.into());
	}

	unsafe {
		if (*cmsg).cmsg_level != libc::SOL_SOCKET || (*cmsg).cmsg_type != libc::SCM_RIGHTS {
			return Err(SocketError::InvalidAddress { reason: "unexpected control message type" }.into());
		}

		let fd = *(libc::CMSG_DATA(cmsg) as *const RawFd);
		Ok(OwnedFd::from_raw_fd(fd))
	}
}


/// A single message to send via sendmmsg.
pub struct SendMsg<'a, A> {
	pub buf: &'a [u8],
	pub addr: &'a A,
}

