#![allow(dead_code)]
use std::net::SocketAddr;

use hashbrown::HashMap;
use message_io::{
    network::{Endpoint, Transport},
    node::NodeHandler,
};

use crate::LOOPBACK;

const RAFT_PORT: u16 = 9000;
const RAFT_COUNT: usize = 5;

pub fn get_peers<Y>(id: usize, handler: NodeHandler<Y>) -> HashMap<usize, Endpoint> {
    (0..RAFT_COUNT)
        .filter(|&i| i != id)
        .map(|i| {
            let out = handler
                .network()
                .connect(
                    Transport::Udp,
                    SocketAddr::from((LOOPBACK, RAFT_PORT + i as u16)),
                )
                .unwrap();
            // dbg!(&out);
            (i, out.0)
        })
        .collect()
}
