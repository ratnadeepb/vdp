//! Client is a representation of a client

use multiqueue::MPMCReceiver;

pub(crate) struct Client<'a> {
	service: &'a str,
	receiver: MPMCReceiver<Mbuf>,
}