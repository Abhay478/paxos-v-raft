//! Code for replica
//! 
//! 

use std::env;
use dc_project::paxos::{dir::replica_init, replica};

fn main() {
    let id = env::args().nth(1).unwrap().parse::<usize>().unwrap();
    println!("Replica {}", id);

    let sock = replica_init(id).unwrap();
    replica::listen(id, sock);
}