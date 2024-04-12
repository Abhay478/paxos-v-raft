//! Code for client.
//!
//! Thinking each client produces a random value at random intervals and sends it to the replica.
//!
//! Which replica? Designated replica, random replica, or all replicas? RANDOM REPLICA.
//!
//! Obviously, the server side of replica should be same for paxos and raft.

use std::env;

use dc_project::{
    paxos::{
        dir::{client_init, get_all_replicas},
        Command, Message,
    },
    Params,
};
use rand::seq::SliceRandom;
use serde_json::to_vec;


/// ```sh
/// cargo run --bin client -- (client_id)
/// ```
/// 
/// Right now only written for Paxos.
fn main() {
    let params = Params::new();
    let reps = get_all_replicas();
    let rep = reps.choose(&mut rand::thread_rng()).unwrap();
    println!("Sending to {:?}", rep);
    let sock = client_init().unwrap();
    let client_id = env::args().nth(1).unwrap().parse::<usize>().unwrap();
    let u = rand::distributions::Uniform::from(0.0..1.0);
    for i in 0..params.k {
        let val = rand::random::<u64>();
        let msg = Message::Request(Command {
            client_id,
            op_id: i,
            op: val.to_string(),
        });
        sock.send_to(&to_vec(&msg).unwrap(), rep).unwrap();
        params.sleep(u, &mut rand::thread_rng());
    }
    // sock.send_to(&to_vec(&Message::Terminate).unwrap(), rep);
    // todo!()
}
