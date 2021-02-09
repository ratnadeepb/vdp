//! The Mempool struct contains a pointer to a DPDK mempool that is guaranteed to be non null

use super::{MemoryError, WrappedCString};
use std::{
	ffi, fmt, mem,
	ptr::{self, NonNull},
};

pub struct Mempool {
	raw: NonNull<dpdk_sys::rte_mempool>,
}

impl Mempool {
	const RX_MBUF_DATA_SIZE: u32 = 2048;
	const RTE_PKTMBUF_HEADROOM: u32 = 128;
	const MBUF_OVERHEAD: u32 =
		mem::size_of::<dpdk_sys::rte_mbuf>() as u32 + Self::RTE_PKTMBUF_HEADROOM;
	const NUM_MBUFS: u32 = 32767; // 2^15 - 1
	const MBUF_SIZE: u32 = Self::RX_MBUF_DATA_SIZE + Self::MBUF_OVERHEAD;
	const MBUF_CACHE_SIZE: u32 = 512;
	const NO_FLAGS: u32 = 0;

	pub fn new(name: &str) -> Result<Self, MemoryError> {
		let n = WrappedCString::to_cstring(name)?;
		let raw = unsafe {
			dpdk_sys::rte_mempool_create(
				n.as_ptr(),
				Self::NUM_MBUFS,
				Self::MBUF_SIZE,
				Self::MBUF_CACHE_SIZE,
				mem::size_of::<dpdk_sys::rte_pktmbuf_pool_private>() as u32,
				Some(dpdk_sys::rte_pktmbuf_pool_init),
				ptr::null::<ffi::c_void>() as *mut _,
				Some(dpdk_sys::rte_pktmbuf_init),
				ptr::null::<ffi::c_void>() as *mut _,
				dpdk_sys::rte_socket_id() as i32,
				Self::NO_FLAGS,
			)
		};
		let mempool = NonNull::new(raw);
		match mempool {
			Some(mem) => {
				log::info!("created mempool: {}", &name);
				Ok(Self { raw: mem })
			}
			None => {
				log::error!("mempool invalid");
				Err(MemoryError::new())
			}
		}
	}

	/// Returns the raw struct pointer
	#[inline]
	pub fn raw(&self) -> &dpdk_sys::rte_mempool {
		unsafe { self.raw.as_ref() }
	}

	/// Returns a mutable raw struct pointer
	#[inline]
	pub fn raw_mut(&mut self) -> &mut dpdk_sys::rte_mempool {
		unsafe { self.raw.as_mut() }
	}

	/// Return mutable reference to the C struct for FFI calls
	/// Does not consume the buffer
	#[inline]
	pub fn get_ptr(&self) -> *mut dpdk_sys::rte_mempool {
		self.raw.as_ptr()
	}

	/// Returns the name of the mempool
	#[inline]
	pub fn name(&self) -> String {
		unsafe {
			ffi::CString::from_raw(&self.raw().name.clone() as *const i8 as *mut i8)
				.to_str()
				.unwrap_or("Name undefined")
				.to_owned()
		}
	}

	pub fn lookup(name: &str) -> Result<Self, MemoryError> {
		let nm = WrappedCString::to_cstring(name)?;
		let r = unsafe { dpdk_sys::rte_mempool_lookup(nm.as_ptr()) };
		match NonNull::new(r) {
			Some(raw) => Ok(Self { raw }),
			None => Err(MemoryError::NoEntries),
		}
	}
}

unsafe impl Sync for Mempool {}
unsafe impl Send for Mempool {}

impl fmt::Debug for Mempool {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let raw = self.raw();
		f.debug_struct(&self.name())
			.field("capacity", &raw.size)
			.field("cache size", &raw.cache_size)
			.field("flags", &format_args!("{:#x}", raw.flags))
			.field("socket", &raw.socket_id)
			.finish()
	}
}

impl Drop for Mempool {
	fn drop(&mut self) {
		unsafe {
			dpdk_sys::rte_mempool_free(self.raw_mut());
		}
	}
}
