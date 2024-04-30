use std::net::SocketAddr;

use message_io::{
    network::{Endpoint, Transport},
    node::{self, NodeHandler, NodeListener},
};

use crate::LOOPBACK;

use super::{leader::Agent, Ballot};

pub const LEADER_PORT: u16 = 4000;
pub const SCOUT_PORT: u16 = 4500;
pub const COMMANDER_PORT: u16 = 5000;
pub const REPLICA_PORT: u16 = 6000;
pub const ACCEPTOR_PORT: u16 = 8000;
pub const CLIENT_PORT: u16 = 9000;

pub const LEADER_COUNT: u8 = 1;
pub const REPLICA_COUNT: u8 = 3;
pub const ACCEPTOR_COUNT: u8 = 3;

pub fn scout_init(acceptors: &Vec<Endpoint>) -> (NodeHandler<Ballot>, NodeListener<Ballot>) {
    let (scout_h, scout_l) = node::split();
    for a in acceptors.iter() {
        scout_h.network().connect(Transport::Udp, a.addr()).unwrap();
    }
    (scout_h, scout_l)
}

pub fn commander_init(acceptors: &Vec<Endpoint>) -> (NodeHandler<()>, NodeListener<()>) {
    let (commander_h, commander_l) = node::split();
    for a in acceptors.iter() {
        commander_h
            .network()
            .connect(Transport::Udp, a.addr())
            .unwrap();
    }
    (commander_h, commander_l)
}

pub fn client_init() -> (NodeHandler<()>, NodeListener<()>) {
    let out = node::split::<()>();
    out
}

pub fn replica_init(id: usize) -> (NodeHandler<()>, NodeListener<()>) {
    let out = node::split::<()>();
    out.0
        .network()
        .listen(
            Transport::Udp,
            SocketAddr::from((LOOPBACK, REPLICA_PORT + id as u16)),
        )
        .unwrap();
    out
}

pub fn leader_init(id: usize) -> (NodeHandler<Agent>, NodeListener<Agent>) {
    let out = node::split();
    out.0
        .network()
        .listen(
            Transport::Udp,
            SocketAddr::from((LOOPBACK, LEADER_PORT + id as u16)),
        )
        .unwrap();
    out
}

pub fn acceptor_init(id: usize) -> (NodeHandler<()>, NodeListener<()>) {
    let out = node::split::<()>();
    out.0
        .network()
        .listen(
            Transport::Udp,
            SocketAddr::from((LOOPBACK, ACCEPTOR_PORT + id as u16)),
        )
        .unwrap();
    out
}

pub fn get_all_leaders<Y>(handler: NodeHandler<Y>) -> Vec<Endpoint> {
    (0..LEADER_COUNT)
        .map(|i| {
            // let temp = node::split::<()>();
            let out = handler
                .network()
                .connect(
                    Transport::Udp,
                    SocketAddr::from((LOOPBACK, LEADER_PORT + i as u16)),
                )
                .unwrap();
            // dbg!(&out);
            out.0
        })
        .collect()

    // todo!()
}

pub fn get_all_replicas<Y>(handler: NodeHandler<Y>) -> Vec<Endpoint> {
    (0..REPLICA_COUNT)
        .map(|i| {
            // SocketAddr::from(([10, 0, 0, i], REPLICA_PORT))
            // let temp = node::split::<()>();
            let out = handler
                .network()
                .connect(
                    Transport::Udp,
                    SocketAddr::from((LOOPBACK, REPLICA_PORT + i as u16)),
                )
                .unwrap();
            // dbg!(&out);
            out.0
        })
        .collect()
}

pub fn get_all_acceptors<Y>(handler: NodeHandler<Y>) -> Vec<Endpoint> {
    (0..ACCEPTOR_COUNT)
        .map(|i| {
            let out = handler
                .network()
                .connect(
                    Transport::Udp,
                    SocketAddr::from((LOOPBACK, ACCEPTOR_PORT + i as u16)),
                )
                .unwrap();
            // dbg!(&out);
            out.0
        })
        .collect()
}
