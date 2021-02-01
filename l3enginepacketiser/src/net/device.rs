//! Create an emulated device
//!
//! This will create a single device on the packetiser process
//! ideally for all network cards in the system

use std::collections::BTreeMap;
use smoltcp::{
	iface::{EthernetInterface, EthernetInterfaceBuilder, NeighborCache},
	phy::{Device, DeviceCapabilities, RxToken, TxToken},
	time::Instant,
	wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address, Ipv4Cidr},
	Result,
};

