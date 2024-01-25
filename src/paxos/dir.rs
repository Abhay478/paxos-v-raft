use std::net::SocketAddr;

const LEADER_PORT: u16 = 8080;
const REPLICA_PORT: u16 = 8081;
const ACCEPTOR_PORT: u16 = 8082;

const LEADER_COUNT: u8 = 9;
const REPLICA_COUNT: u8 = 9;
const ACCEPTOR_COUNT: u8 = 9;

pub fn get_all_leaders() -> Vec<SocketAddr> {
    (1..LEADER_COUNT)
        .map(|i| SocketAddr::from(([10, 0, 0, i], LEADER_PORT)))
        .collect()
}

pub fn get_all_replicas() -> Vec<SocketAddr> {
    (1..REPLICA_COUNT)
        .map(|i| SocketAddr::from(([10, 0, 0, i], REPLICA_PORT)))
        .collect()
}

pub fn get_all_acceptors() -> Vec<SocketAddr> {
    (1..ACCEPTOR_COUNT)
        .map(|i| SocketAddr::from(([10, 0, 0, i], ACCEPTOR_PORT)))
        .collect()
}
