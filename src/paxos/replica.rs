#![allow(dead_code)]
use self::dir::get_all_leaders;
use serde_derive::{Deserialize, Serialize};
use serde_json::{from_slice, to_vec};
use std::{
    collections::BTreeMap, net::{SocketAddr, UdpSocket}
};
use hashbrown::HashMap;

use super::*;

const WINDOW: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, Copy)]
pub struct ReplicaState {
    n: usize,
}

impl ReplicaState {
    pub fn triv(s: String) -> impl Fn (&ReplicaState) -> (ReplicaState, Result<String, String>) {
        move |q| (*q, Ok(s.clone()))
    }
}
/// This can be something as simple as
/// ```
/// |q: ReplicaState| (q, Ok(""))
/// ```
/// in which case we'd be storing constants and not operations.
/// 
/// The triv() function does just that.

// #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Op {
    client_id: usize,
    op_id: usize,
    op: Box<dyn Fn(&ReplicaState) -> (ReplicaState, Result<String, String>) + Send + Sync>,
}
/* : Debug + Clone + PartialEq + Eq + std::hash::Hash + serde::Serialize + serde::de::DeserializeOwned */

pub struct Replica {
    id: usize,
    state: ReplicaState,
    slot_in: usize,
    slot_out: usize,
    // requests: BTreeMap<Uuid, Command>,
    requests: Vec<Command>,
    proposals: BTreeMap<usize, Command>,
    decisions: HashMap<usize, Command>,
    leaders: Vec<SocketAddr>,
    sock: UdpSocket,
    clients: HashMap<usize, SocketAddr>,
}

impl Replica {
    fn new(id: usize, init: ReplicaState, leaders: Vec<SocketAddr>, sock: UdpSocket) -> Self {
        Self {
            id,
            state: init,
            slot_in: 1,
            slot_out: 1,
            requests: vec![],
            proposals: BTreeMap::new(),
            decisions: HashMap::new(),
            leaders,
            sock,
            clients: HashMap::new(),
        }
    }

    fn propose(&mut self) {
        while self.slot_in < self.slot_out + WINDOW && !self.requests.is_empty() {
            if self.decisions.get(&self.slot_in).is_none() {
                let c = self.requests.pop().unwrap();
                self.proposals.insert(self.slot_in, c.clone());
                let msg = Message::Propose(
                    self.slot_in,
                    c,
                );
                // self.proposals[&self.slot_in] = c;
                let buf = to_vec(&msg).unwrap();

                self.leaders.iter().for_each(|addr| {
                    self.sock.send_to(&buf, addr).unwrap();
                });
            }
            self.slot_in += 1;
        }
    }

    fn perform(&mut self, op: Command) {
        /* 
            NOTE:
            - Pseudoocode has this particular if block so as to avoid duplicate executions in case one command is decided at multiple slots. 
            - Since all replicas have the same sequence of decisions, this is merely an optimisation.
            - We contend that a command may mutate some external state, and hence is not idempotent.
            - Thus, this block has been commented out.
        */

        // if self.decisions.contains(&Some(op)) {
        //     self.slot_out += 1;
        //     return;
        // }

        let addr = self.clients.get(&op.client_id).unwrap();
        let (state, res) = ReplicaState::triv(op.op)(&self.state);
        // For some reason, this should be atomic, but since we're not using threads, it's fine.
        {
            // let _un = self.lock.lock().unwrap();
            self.state = state;
            self.slot_out += 1;
        }
        let msg = Message::Response(op.op_id, "Hello there".to_string(), res);
        let buf = to_vec(&msg).unwrap();
        self.sock.send_to(&buf, addr).unwrap();
    }
}

fn listen(id: usize, sock: UdpSocket) {
    let leaders = get_all_leaders();
    let mut rep = Replica::new(id, ReplicaState::default(), leaders, sock);
    loop {
        let mut buf = vec![];
        let (l, src) = rep.sock.recv_from(&mut buf).unwrap();
        let msg = from_slice::<Message>(&buf[..l]).unwrap();
        match msg {
            Message::Request(c) => {
                let c = c.clone();
                let _ = rep.clients.try_insert(c.client_id, src);
                rep.requests.push(c);
            }
            Message::Decision(slot, command) => {
                rep.decisions.insert(slot, command);
                while let Some(c1) = rep.decisions.get(&rep.slot_out) {
                    if let Some(c2) = rep.proposals.remove(&rep.slot_out) {
                        if c2 != *c1 {
                            rep.requests.push(c2);
                        }
                    }
                    rep.perform(c1.clone()); // GAH, CLONES!
                }
            }
            _ => unreachable!(),
        }
        rep.propose();
    }
}
