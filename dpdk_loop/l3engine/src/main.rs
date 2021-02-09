pub mod apis;

pub use apis::*;

use crate::{eal_cleanup, eal_init, Mbuf, Mempool, Port};
use log;
use std::sync::{
	atomic::{AtomicBool, Ordering},
	Arc,
};

const G_MEMPOOL_NAME: &str = "GLOBAL_MEMPOOL";
const QUEUE_SZ: usize = 32;

/// Handle Ctrl+C
fn handle_signal(kr: Arc<AtomicBool>) {
	ctrlc::set_handler(move || {
		kr.store(false, Ordering::SeqCst);
	})
	.expect("Error setting Ctrl-C handler");
}

fn recv_pkts(port: &Port, in_pkts: &mut Vec<Mbuf>) -> usize {
	let queue_id = unsafe { dpdk_sys::_rte_lcore_id() as u16 };
	in_pkts.extend(port.receive(queue_id, in_pkts.len()));

	in_pkts.len()
}

fn xmit_pkts(port: &Port, out_pkts: &mut Vec<Mbuf>) -> usize {
	let queue_id = unsafe { dpdk_sys::_rte_lcore_id() as u16 };
	let num = port.send(out_pkts, queue_id ^ 1);
	out_pkts.clear(); // deallocate all buffers
	num
}

fn main() {
	log::info!("Initializing DPDK env ...");
	let args = vec![
		String::from("-l 0-1"),
		String::from("-n 4"),
		String::from("--proc-type=primary"),
		String::from("--base-virtaddr=0x7f000000000"),
		String::from("--"),
		String::from("-p 3"),
		String::from("-n 2"),
	];
	#[cfg(debug)]
	println!("main process args: {:?}", &args);
	eal_init(args).unwrap();

	#[cfg(feature = "debug")]
	println!("environment initialised");

	let cores = vec![0];

	log::info!("setup mempool");
	let mempool;
	match Mempool::new(G_MEMPOOL_NAME) {
		Ok(mp) => {
			#[cfg(feature = "debug")]
			println!("mempool address: {:p}", mp.get_ptr());
			mempool = mp;
		}
		Err(e) => panic!("Failed to initialize mempool: {}", e),
	}

	#[cfg(feature = "debug")]
	println!("mempool set");

	log::info!("setup ports");
	#[cfg(feature = "debug")]
	println!("setup ports");
	let eth_devs = "port0";
	let mut port = Port::new(eth_devs, 0u16).unwrap();
	port.configure(cores.len() as u16, &mempool).unwrap();
	port.start().unwrap();

	#[cfg(feature = "debug")]
	println!("ports set");

	// hold packets received from outside and packetiser
	let mut in_pkts: Vec<Mbuf> = Vec::with_capacity(QUEUE_SZ);
	// let mut out_pkts: Vec<Mbuf> = Vec::with_capacity(QUEUE_SZ);

	// handling Ctrl+C
	let keep_running = Arc::new(AtomicBool::new(true));
	// let kr = keep_running.clone();
	handle_signal(keep_running.clone());

	#[cfg(feature = "debug")]
	println!("main: secondary started");
	while keep_running.load(Ordering::SeqCst) {
		let _rsz = recv_pkts(&port, &mut in_pkts);
		#[cfg(feature = "debug")]
		println!("Received {} packets", _rsz);
		let _tsz = xmit_pkts(&port, &mut in_pkts);
		#[cfg(feature = "debug")]
		println!("Received {} packets", _rsz);
	}

	#[cfg(feature = "debug")]
	println!("main: stopping");
	unsafe { dpdk_sys::_pkt_stop_and_close_ports() };
	#[cfg(feature = "debug")]
	println!("main: ports closed");
	eal_cleanup(&mempool).unwrap();
}
