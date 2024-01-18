#![allow(dead_code)]
use serde_derive::{Deserialize, Serialize};
use serde_json::{from_slice, to_vec};
use uuid::Uuid;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    net::{SocketAddr, UdpSocket}, str::FromStr,
};
use self::dir::get_all_leaders;

use super::*;
// use uuid::Uuid;

// use crate::{
//     dir::get_all_leaders,
//     models::{Command, Message},
// };

const WINDOW: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, Copy)]
pub struct ReplicaState {
    n: usize,
}
/// This can be something as simple as
/// ```
/// |q: ReplicaState| (q, Ok(1))
/// ```
/// in which case we'd be storing constants and not operations.

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
    requests: HashMap<Uuid, Op>, // All requests ever
    pending: Vec<Uuid>, // Remove from here each time you propose
    proposals: Vec<Option<Uuid>>, 
    decisions: Vec<Option<Uuid>>,
    leaders: Vec<SocketAddr>,
    sock: UdpSocket,
    clients: BTreeMap<usize, SocketAddr>,
}

impl Replica {
    fn new(id: usize, init: ReplicaState, leaders: Vec<SocketAddr>, sock: UdpSocket) -> Self {
        Self {
            id,
            state: init,
            slot_in: 1,
            slot_out: 1,
            requests: HashMap::new(),
            pending: vec![],
            proposals: vec![],
            decisions: vec![],
            leaders,
            sock,
            clients: BTreeMap::new(),
        }
    }

    fn propose(&mut self) {
        while self.slot_in < self.slot_out + WINDOW && !self.pending.is_empty() {
            if self.decisions.get(self.slot_in).is_none() {
                let c = self.pending.pop().unwrap();
                let c = (c, self.requests.get(&c).unwrap());
                let msg = Message::Propose(
                    self.slot_in,
                    Command {
                        client_id: c.1.client_id,
                        op_id: c.1.op_id,
                        op: c.0.to_string(),
                    },
                );
                // self.proposals.insert(self.slot_in, c.0);
                self.proposals[self.slot_in] = Some(c.0);
                let buf = to_vec(&msg).unwrap();

                self.leaders.iter().for_each(|addr| {
                    self.sock.send_to(&buf, addr).unwrap();
                });
            }
            self.slot_in += 1;
        }
    }

    fn perform(&mut self, op: Uuid) {
        if self.decisions.contains(&Some(op)) {
            self.slot_out += 1;
            return;
        }
        let op = self.requests.get(&op).unwrap();
        let addr = self.clients.get(&op.client_id).unwrap();
        let (state, res) = (op.op)(&self.state);
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

fn listen(
    id: usize,
    sock: UdpSocket,
) {
    let leaders = get_all_leaders();
    let mut rep = Replica::new(id, ReplicaState::default(), leaders, sock);
    loop {
        let mut buf = vec![];
        let (l, src) = rep.sock.recv_from(&mut buf).unwrap();
        let msg = from_slice::<Message>(&buf[..l]).unwrap();
        match msg {
            Message::Request(c) => {
                let c = c.clone();
                let op = Op {
                    client_id: c.client_id,
                    op_id: c.op_id,
                    op: Box::new(move |q| {
                        let q = ReplicaState { n: q.n + 1 };
                        (q, Ok(c.op.clone())) // CLONE!!
                    }),
                };
                rep.requests.insert(Uuid::new_v4(), op);
                rep.clients.insert(c.client_id, src);
            }
            Message::Decision(slot, command) => {
                if slot > rep.decisions.len() {
                    rep.decisions.resize(slot, None);
                }
                rep.decisions[slot] = Some(Uuid::from_str(&command.op).unwrap());
                while let Some(c1) = rep.decisions[rep.slot_out] {
                    if let Some(c2) = rep.proposals.remove(rep.slot_out) {
                        if c2 != c1 {
                            rep.pending.push(c2);
                        }
                    }
                    rep.perform(c1);
                }
            }
            _ => unreachable!(),
        }
        rep.propose();
    }
}
