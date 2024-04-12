#![allow(dead_code)]
use self::dir::get_all_leaders;
use hashbrown::HashMap;
use serde_derive::{Deserialize, Serialize};
use serde_json::{from_slice, to_vec}; // Might have to change this to bincode or a custom impl.
use std::{
    collections::BTreeMap,
    net::{SocketAddr, UdpSocket},
};

use super::*;

const WINDOW: usize = 32;

/// Right now this is just a `usize`, but it can really be anything. The rest of the code is general enough.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, Copy)]
pub struct ReplicaState {
    n: usize,
}

impl ReplicaState {
    pub fn triv(s: String) -> impl Fn(&ReplicaState) -> (ReplicaState, Result<String, String>) {
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
pub struct Op {
    /// Change to SocketAddr?
    client_id: usize,
    /// Sequence number.
    op_id: usize,
    /// The operation to be performed.
    op: Box<dyn Fn(&ReplicaState) -> (ReplicaState, Result<String, String>) + Send + Sync>,
}

/// Node struct.
pub struct Replica {
    /// Just a lil number. Unique among all replicas.
    id: usize,
    /// Eh, just some state.
    state: ReplicaState,
    /// Things for the algorithm.
    slot_in: usize,
    slot_out: usize,
    /// Outstanding requests from clients
    requests: Vec<Command>,
    /// Outstaning proposals that have been sent out, but not decided upon.
    proposals: BTreeMap<usize, Command>,
    /// These are the done deals.
    decisions: HashMap<usize, Command>,

    /// These are the guys you gotta talk to.
    leaders: Vec<SocketAddr>,

    /// This is us.
    sock: UdpSocket,

    /// These are those icky clients that keep bothering us.
    clients: HashMap<usize, SocketAddr>,
}

impl Replica {
    pub fn new(id: usize, leaders: Vec<SocketAddr>, sock: UdpSocket) -> Self {
        Self {
            id,
            state: ReplicaState::default(),
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

    /// Self explanatory name.
    ///
    /// Each proposal is removed from `requests`, topped off with a slot, and sent to all leaders.
    /// This is done for multiple requests, each getting a different slot.
    fn propose(&mut self) {
        while self.slot_in < self.slot_out + WINDOW && !self.requests.is_empty() {
            if self.decisions.get(&self.slot_in).is_none() {
                let c = self.requests.pop().unwrap(); // do this
                self.proposals.insert(self.slot_in, c.clone()); // and then do that
                let msg = Message::Propose(self.slot_in, c); // And the this.

                // self.proposals[&self.slot_in] = c;

                let buf = to_vec(&msg).unwrap();

                // Now send the bloody thing
                self.leaders.iter().for_each(|addr| {
                    self.sock.send_to(&buf, addr).unwrap();
                });
            }
            self.slot_in += 1;
        }
    }

    /// Simple pipeline.
    /// Gets thing from leader, sends thing to client.
    /// Shimpul.
    fn perform(&mut self, op: Command) {
        /*
            NOTE:
            - Pseudoocode has this particular if block so as to avoid duplicate executions in case one command is decided at multiple slots.
            - Since all replicas have the same sequence of decisions, this is merely an optimisation.
            - We contend that a command may mutate some external state, and hence is not idempotent.
            - Thus, this block has been commented out.
            - We *are* keeping this, just in case.
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
        // TODO: Change the contents of Message::Response, maybe. Don't think String is enough.
        let msg = Message::Response(op.op_id, "Hello there".to_string(), res);

        let buf = to_vec(&msg).unwrap();
        self.sock.send_to(&buf, addr).unwrap();
    }
}

/// This is the main loop for the replica. It listens for messages from the leaders and clients.
pub fn listen(id: usize, sock: UdpSocket) {
    let leaders = get_all_leaders();
    let mut rep = Replica::new(id, leaders, sock);
    loop {
        let mut buf = vec![];
        let (l, src) = rep.sock.recv_from(&mut buf).unwrap();

        // Verify that only **ONE** message is received.
        let msg = from_slice::<Message>(&buf[..l]).unwrap();
        match msg {
            Message::Request(c) => {
                let c = c.clone();
                let _ = rep.clients.try_insert(c.client_id, src);
                rep.requests.push(c);
            }
            Message::Decision(slot, command) => {
                // Accept the consensus.
                rep.decisions.insert(slot, command);
                while let Some(c1) = rep.decisions.get(&rep.slot_out) {
                    if let Some(c2) = rep.proposals.remove(&rep.slot_out) {
                        if c2 != *c1 {
                            rep.requests.push(c2);
                        }
                    }

                    // Actually do the thing.
                    rep.perform(c1.clone()); // GAH, CLONES!
                }
            }
            _ => unreachable!(), // It had better be, damn it.
        }
        rep.propose();
    }
}
