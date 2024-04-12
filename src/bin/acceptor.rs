//! Code for acceptor.
//! 
//! 

use std::env;
use dc_project::paxos::{dir::acceptor_init, acceptor};

fn main() {
    let id = env::args().nth(1).unwrap().parse::<usize>().unwrap();
    println!("Acceptor {}", id);

    let sock = acceptor_init(id).unwrap();
    acceptor::listen(id, sock);
}