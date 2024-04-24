//! Code for acceptor.
//!
//!

use dc_project::paxos::{acceptor, dir::acceptor_init};
use std::env;

fn main() {
    let id = env::args().nth(1).unwrap().parse::<usize>().unwrap();
    println!("Acceptor {}", id);

    let sock = acceptor_init(id);
    acceptor::listen(id, sock.1, sock.0);
}
