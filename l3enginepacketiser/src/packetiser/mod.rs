//! This module defines a routing table and the packetiser struct
//!
//! The RoutingTable maintains a map between client IDs and IPs assigned to them
//!
//! The Packetiser runs the prime secondary DPDK thread.
//! This thread talks to the primary and gets packets that are not ARP and not dropped by the primary
//! This thread is responsible for applying L3 network policies on these packets,
//! register clients
//! and to route them to and from clients
//!
//!
//! This is the first client we are building.
//! The design is that the main DPDK process only handles incoming and outgoing packets
//! Incoming packets are handed over to a secondary DPDK process
//! This module performs the basic processing and sends them over to the clients
//! It will also receive the modules from the clients and send them to main process
//!
//! This module will ultimately hold filters that can be enabled to apply certain policies

const QUEUE_SZ: usize = 512;
pub(crate) struct Packetiser {
	channel: Channel, // receive and transmit packets from and to the main process
	mempool: Mempool, // mempool to use
	in_queue: ArrayQ, // packets that have been received from the primary process
	pub(crate) o_bufqueue: SegQueue<Mbuf>, // packets that have been received from clients
	cap: usize,                            // number of packets to be held in the buffers at any time
	allocated_ids: Vec<u16>,               // hold all ids that have been allocated
}