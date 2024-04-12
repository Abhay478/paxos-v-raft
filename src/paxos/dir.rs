use std::{
    io,
    net::{SocketAddr, UdpSocket},
};

const LEADER_PORT: u16 = 8080;
const REPLICA_PORT: u16 = 8081;
const ACCEPTOR_PORT: u16 = 8082;
const CLIENT_PORT: u16 = 8083;

const LEADER_COUNT: u8 = 9;
const REPLICA_COUNT: u8 = 9;
const ACCEPTOR_COUNT: u8 = 9;

pub fn client_init() -> Result<UdpSocket, io::Error> {
    UdpSocket::bind(format!("0.0.0.0:{CLIENT_PORT}").parse::<String>().unwrap())
}

pub fn replica_init(id: usize) -> Result<UdpSocket, io::Error> {
    UdpSocket::bind(SocketAddr::from(([10, 0, 0, id as u8], REPLICA_PORT)))
}

pub async fn leader_init(id: usize) -> Result<tokio::net::UdpSocket, io::Error> {
    tokio::net::UdpSocket::bind(SocketAddr::from(([10, 0, 0, id as u8], LEADER_PORT))).await
}

pub fn acceptor_init(id: usize) -> Result<UdpSocket, io::Error> {
    UdpSocket::bind(SocketAddr::from(([10, 0, 0, id as u8], ACCEPTOR_PORT)))
}

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
