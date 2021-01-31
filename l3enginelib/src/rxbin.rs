//! This module is responsible for the two functions that get packets into the main engine
//!
//! There are two sources of packets for the main engine:
//! 	1. External packets from the NIC
//! 	2. Internal packets from the packetiser

use crate::{FROM_PACKETISER, MEMPOOL, OUT_PKTS, PROCESSOR_THREAD, TO_PACKETISER};
use crossbeam_queue::ArrayQueue;
use l3enginelib::apis::{Mbuf, Mempool, Port};
use state::Storage;

