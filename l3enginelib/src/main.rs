
use state::Storage;
use crossbeam_queue::ArrayQueue;
use l3enginelib::{Channel, Mbuf, Mempool, Port, eal_cleanup, eal_init};
use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}};
use log;
use zmq::Context;

const G_MEMPOOL_NAME: &str = "GLOBAL_MEMPOOL";

const QUEUE_SZ: usize = 32;

const PACKETISER_ZMQ_PORT: &str = "tcp://*:5555";

/// A central mempool
///
/// DPDK mempool is based on DPDK's lockless ring and thus thread-safe
pub(crate) static MEMPOOL: Storage<Mempool> = Storage::new();

/// Unbounded queue to hold packets that are to sent immediately out
pub(crate) static OUT_PKTS: Storage<ArrayQueue<Mbuf>> = Storage::new();
/// Unbounded queue to hold packets that are to sent to the packetiser
pub(crate) static TO_PACKETISER: Storage<ArrayQueue<Mbuf>> = Storage::new();
/// Unbounded queue to hold packets that are coming from the packetiser
pub(crate) static FROM_PACKETISER: Storage<ArrayQueue<Mbuf>> = Storage::new();
/// Channel to communicate between engine and packetiser
pub(crate) static CHANNEL: Storage<Channel> = Storage::new();

fn handle_signal(kr: Arc<AtomicBool>) {
	ctrlc::set_handler(move || {
		kr.store(false, Ordering::SeqCst);
	})
	.expect("Error setting Ctrl-C handler");
}

/// Get packets from the outside world
fn recv_pkts(ports: &Vec<Port>) -> usize {
	let queue_id = unsafe { dpdk_sys::_rte_lcore_id() as u16 };
	let ring_pkts = TO_PACKETISER.get();
	let sz = ring_pkts.capacity() - ring_pkts.len();
	let pkts = ports[0].receive(queue_id, sz);
	let len = pkts.len();

	for pkt in pkts {
		ring_pkts.push(pkt).unwrap(); // we always check for space beforehand
	}
	len
}

/// Send packets to the packetiser
fn send_to_packetiser() -> usize {
	let ch = CHANNEL.get();
	let ring_pkts = TO_PACKETISER.get();
	if ring_pkts.is_empty() {
		return 0usize;
	}
	
	let mut len = 0;
	match ring_pkts.pop() {
		Some(pkt) => {
			len += 1;
			match ch.send_to_packetiser(pkt) {
				Ok(_) => {},
				Err(m) => {
					log::error!("Error sending it: {}", m);
				}
			}
		},
		None => {},
	}
	len
}

/// Get packets from the packetiser
fn transmit_pkts(port: Port) -> usize {
	let out_pkts = FROM_PACKETISER.get();

	if out_pkts.is_empty() {
		return 0usize;
	}

	if !out_pkts.is_full() {
		let len = out_pkts.capacity() - out_pkts.len();
		let ch = CHANNEL.get();
		let mp = MEMPOOL.get();
		let mut pkts = Vec::with_capacity(len);
		for l in 0..len {
			match Mbuf::new(mp) {
				Ok(buf) => out_pkts.push(buf).unwrap(),
				Err(_) => {
					pkts.truncate(l);
					break;
				}
			}
		}
		ch.recv_from_packetiser_bulk(&mut pkts, len);
	}

	// REVIEW: Can this conversion be done away with?
	let mut pkts = Vec::with_capacity(out_pkts.len());
	loop {
		match out_pkts.pop() {
			Some(buf) => pkts.push(buf),
			None => break,
		}
	}

	let queue_id = unsafe { dpdk_sys::_rte_lcore_id() as u16 };
	port.send(pkts, queue_id ^ 1)
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
	let cur_core = unsafe { dpdk_sys::_rte_lcore_id() };

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
	MEMPOOL.set(mempool);
	#[cfg(feature = "debug")]
	println!("mempool set");

	log::info!("setup ports");
	let eth_devs = vec!["port0"];
	let mut ports: Vec<Port> = Vec::new();
	{
		let mempool = MEMPOOL.get();
		let mut p = Port::new(eth_devs[0], 0u16).unwrap();
		p.configure(cores.len() as u16, mempool).unwrap();
		p.start().unwrap();
		ports.push(p);
	} // MEMPOOL released

	#[cfg(feature = "debug")]
	println!("ports set");

	#[cfg(feature = "debug")]
	println!("main: waiting for secondary");
	let context = Context::new();
	let responder = context.socket(zmq::REP).unwrap(); // fatal error
	assert!(responder.bind(PACKETISER_ZMQ_PORT).is_ok());
	let mut msg = zmq::Message::new();
	responder.recv(&mut msg, 0).unwrap();

	// start the channel
	let ch = Channel::new().unwrap(); // we can't work otherwise!
	CHANNEL.set(ch);

	// packets to be sent out
	OUT_PKTS.set(ArrayQueue::new(QUEUE_SZ));
	// packets to be sent to the packetiser
	TO_PACKETISER.set(ArrayQueue::new(QUEUE_SZ));
	// packets to be received from the packetiser
	FROM_PACKETISER.set(ArrayQueue::new(QUEUE_SZ));

	// handling Ctrl+C
	let keep_running = Arc::new(AtomicBool::new(true));
	let kr = keep_running.clone();
	handle_signal(keep_running.clone());

	#[cfg(feature = "debug")]
	println!("main: secondary started");
	while kr.load(Ordering::SeqCst) {

	}
}