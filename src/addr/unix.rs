use crate::{Domain};
use crate::addr::ToSockAddr;

/// Unix domain socket address (file path).

/// Unix domain socket marker.
///
/// Sockets with this domain use filesystem paths (e.g., /tmp/app.sock).
/// Only works on the same machine.
pub struct Unix;
/// Unix domain socket address (file path or abstract).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnixAddr {
	path: Vec<u8>,
	/// True if this is an abstract socket (Linux-only, no filesystem entry).
	is_abstract: bool,
}

impl UnixAddr {
	/// Creates a new Unix address from a filesystem path.
	pub fn new<P: AsRef<[u8]>>(path: P) -> Self {
		Self {
			path: path.as_ref().to_vec(),
			is_abstract: false,
		}
	}
	
	/// Creates from a string path.
	pub fn from_str(path: &str) -> Self {
		Self {
			path: path.as_bytes().to_vec(),
			is_abstract: false,
		}
	}
	
	/// Creates an abstract socket address (Linux-only).
	///
	/// Abstract sockets exist only in memory — no filesystem entry.
	/// Auto-removed when all references close. No permission issues.
	/// Name can contain any bytes (no null-terminator needed).
	pub fn abstract_socket<P: AsRef<[u8]>>(name: P) -> Self {
		Self {
			path: name.as_ref().to_vec(),
			is_abstract: true,
		}
	}
	
	/// Returns true if this is an abstract socket.
	pub fn is_abstract(&self) -> bool {
		self.is_abstract
	}
	
	/// Returns the path bytes.
	pub fn path(&self) -> &[u8] {
		&self.path
	}
	
	/// Converts to the raw sockaddr_un for syscalls.
	pub(crate) fn to_raw(&self) -> Option<libc::sockaddr_un> {
		let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
		addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
		
		if self.is_abstract {
			// Abstract: first byte is null, then the name
			if self.path.len() + 1 >= addr.sun_path.len() {
				return None;
			}
			// sun_path[0] is already 0 from zeroed()
			for (i, &byte) in self.path.iter().enumerate() {
				addr.sun_path[i + 1] = byte as libc::c_char;
			}
		} else {
			// Filesystem path: null-terminated
			if self.path.len() >= addr.sun_path.len() {
				return None;
			}
			for (i, &byte) in self.path.iter().enumerate() {
				addr.sun_path[i] = byte as libc::c_char;
			}
		}
		
		Some(addr)
	}
	
	/// Creates from raw sockaddr_un.
	pub(crate) fn from_raw(raw: &libc::sockaddr_un) -> Self {
		// Check if abstract (first byte is null but there's more data)
		if raw.sun_path[0] == 0 {
			// Abstract socket — find the end
			let len = raw.sun_path[1..]
				.iter()
				.position(|&c| c == 0)
				.unwrap_or(raw.sun_path.len() - 1);
			
			let path: Vec<u8> = raw.sun_path[1..=len]
				.iter()
				.map(|&c| c as u8)
				.collect();
			
			Self { path, is_abstract: true }
		} else {
			// Filesystem path
			let len = raw.sun_path
				.iter()
				.position(|&c| c == 0)
				.unwrap_or(raw.sun_path.len());
			
			let path: Vec<u8> = raw.sun_path[..len]
				.iter()
				.map(|&c| c as u8)
				.collect();
			
			Self { path, is_abstract: false }
		}
	}
}



/*
- No port — Unix sockets don't use ports
- Path instead — like /tmp/app.sock
- Vec<u8> — paths can vary in length

Why Option? Unix socket paths have a maximum length
(typically 108 bytes, with one reserved for null terminator).
If the path is too long, we return None rather than silently
truncating or causing undefined behavior.
*/

impl ToSockAddr for UnixAddr {
	fn with_raw<F, R>(&self, f: F) -> Option<R>
	where
		F: FnOnce(*const libc::sockaddr, libc::socklen_t) -> R,
	{
		let raw = self.to_raw()?;  // Returns None if path too long
		let ptr = &raw as *const _ as *const libc::sockaddr;
		let len = std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;
		Some(f(ptr, len))
	}
}
impl Domain for Unix {
	type Addr = UnixAddr;
	
	#[inline]
	fn raw() -> libc::c_int {
		libc::AF_UNIX
	}
}