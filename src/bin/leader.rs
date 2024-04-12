//! Code for paxos leader
//! 

use std::env;

use dc_project::paxos::{dir::leader_init, leader};
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() {
    let id = env::args().nth(1).unwrap().parse::<usize>().unwrap();
    println!("Leader {}", id);

    let sock: UdpSocket = leader_init(id).await.unwrap();
    leader::listen(id, sock).await;
}