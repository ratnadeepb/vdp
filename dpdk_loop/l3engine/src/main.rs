pub mod apis;

pub use apis::*;

use crate::{eal_cleanup, eal_init, Mbuf, Mempool, Port};
use log;
use std::{
	net::Ipv4Addr,
	slice::from_raw_parts_mut,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
};

use pnet::{
	datalink::MacAddr,
	packet::{
		arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket},
		ethernet::{EtherTypes, MutableEthernetPacket},
		MutablePacket, Packet,
	},
};

use etherparse::{InternetSlice, LinkSlice, ReadError, SlicedPacket, TransportSlice};

const G_MEMPOOL_NAME: &str = "GLOBAL_MEMPOOL";
const QUEUE_SZ: usize = 32;

const ETHERTYPE_ARP: u16 = 0x0806;

struct FiveTuple {
	src_mac: [u8; 6],
	src_ip: Ipv4Addr,
	dst_mac: [u8; 6],
	dst_ip: Ipv4Addr,
	ethertype: u16,
}

pub(crate) fn parse_pkt(pkt: &[u8], local_ip: Ipv4Addr, local_mac: [u8; 6]) -> Option<FiveTuple> {
	let sliced_pkt = SlicedPacket::from_ethernet(pkt).unwrap();
	let linkslice = sliced_pkt.link.unwrap();
	let ipslice = sliced_pkt.ip.unwrap();
	let transportslice = sliced_pkt.transport.unwrap();
	let link = match linkslice {
		LinkSlice::Ethernet2(l) => l.to_header(),
	};
	let ip = match ipslice {
		InternetSlice::Ipv4(ip) => ip,
		_ => return None,
	};
	let transport = match transportslice {
		TransportSlice::Tcp(tcp) => tcp,
		TransportSlice::Udp(_) => return None,
	};
	let src_mac = link.source;
	let dst_mac = link.destination;
	let ethertype = link.ether_type;
	let src_ip = ip.source_addr();
	let dst_ip = ip.destination_addr();
	// let dst_port = transport.destination_port();
	if dst_ip != local_ip {
		return None;
	}

	if dst_mac != local_mac {
		return None;
	}

	// if !ports.contains(&dst_port) {
	// 	return Err(FiveTupleErr::WrongPort);
	// }

	Some(FiveTuple {
		src_ip,
		src_mac,
		dst_mac,
		dst_ip,
		ethertype,
	})
}

fn arr_to_macaddr(mac: [u8; 6]) -> MacAddr {
	MacAddr::new(mac[0], mac[1], mac[2], mac[3], mac[4], mac[5])
}

fn handle_arp(
	local_ip: Ipv4Addr,
	local_mac: [u8; 6],
	dst_mac: [u8; 6],
	dst_ip: Ipv4Addr,
	mp: &Mempool,
	ethertype: u16,
) -> Option<Mbuf> {
	if local_mac != dst_mac || local_ip != dst_ip {
		return None;
	}

	if ethertype.to_be() == ETHERTYPE_ARP {
		if local_mac == dst_mac || local_ip == dst_ip {
			let mut eth_buf = [0u8; 42];
			let mut eth_pkt = MutableEthernetPacket::new(&mut eth_buf)?;
			eth_pkt.set_destination(MacAddr::broadcast());
			let source_mac = arr_to_macaddr(local_mac);
			eth_pkt.set_source(source_mac);
			eth_pkt.set_ethertype(EtherTypes::Arp);
			let mut arp_buffer = [0u8; 28];
			let mut arp_packet = MutableArpPacket::new(&mut arp_buffer)?;
			arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
			arp_packet.set_protocol_type(EtherTypes::Ipv4);
			arp_packet.set_hw_addr_len(6);
			arp_packet.set_proto_addr_len(4);
			arp_packet.set_operation(ArpOperations::Request);
			arp_packet.set_sender_hw_addr(source_mac);
			arp_packet.set_sender_proto_addr(local_ip);
			arp_packet.set_target_hw_addr(MacAddr::zero());
			arp_packet.set_target_proto_addr(dst_ip);

			eth_pkt.set_payload(arp_packet.packet_mut());

			match Mbuf::from_bytes(eth_pkt.packet(), mp) {
				Ok(out_buf) => return Some(out_buf),
				Err(_) => {}
			}
		}
	}
	None
}

/// Handle Ctrl+C
fn handle_signal(kr: Arc<AtomicBool>) {
	ctrlc::set_handler(move || {
		kr.store(false, Ordering::SeqCst);
	})
	.expect("Error setting Ctrl-C handler");
}

fn recv_pkts(port: &[Port; 2], in_pkts: &mut Vec<Mbuf>) -> usize {
	let queue_id = unsafe { dpdk_sys::_rte_lcore_id() as u16 };
	in_pkts.extend(port[0].receive(queue_id, in_pkts.len()));
	// in_pkts.extend(port[1].receive(queue_id, in_pkts.len()));

	#[cfg(feature = "debug")]
	if in_pkts.len() > 0 {
		println!("got {} pkts", in_pkts.len());
	}

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
	let eth_dev1 = "port0";
	let mut port1 = Port::new(eth_dev1, 0u16).unwrap();
	port1.configure(cores.len() as u16, &mempool).unwrap();
	port1.start().unwrap();
	let eth_dev2 = "port1";
	let mut port2 = Port::new(eth_dev2, 1u16).unwrap();
	port2.configure(cores.len() as u16, &mempool).unwrap();
	port2.start().unwrap();
	let ports = [port1, port2];

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
		let rsz = recv_pkts(&ports, &mut in_pkts);
		#[cfg(feature = "debug")]
		if rsz > 0 {
			println!("Received {} packets", rsz);
		}
		if rsz > 0 {
			let _tsz = xmit_pkts(&port2, &mut in_pkts);
			#[cfg(feature = "debug")]
			println!("Transmitted {} packets", _tsz);
			for pkt in &in_pkts {
				let local_ip = Ipv4Addr::new(192, 168, 1, 2);
				let mac = [0x90, 0xe2, 0xba, 0x87, 0x6b, 0xe8];
				let buf =
					unsafe { from_raw_parts_mut(dpdk_sys::_pkt_raw_addr(pkt.get_ptr()), 1536) };
				let ft = parse_pkt(buf, local_ip, mac).unwrap();
				let mut mbuf = vec![handle_arp(
					local_ip,
					mac,
					ft.src_mac,
					ft.src_ip,
					&mempool,
					ft.ethertype,
				)
				.unwrap()];
				xmit_pkts(&port1, &mut mbuf);
			}
		}
	}

	#[cfg(feature = "debug")]
	println!("main: stopping");
	unsafe { dpdk_sys::_pkt_stop_and_close_ports() };
	#[cfg(feature = "debug")]
	println!("main: ports closed");
	eal_cleanup(&mempool).unwrap();
}
