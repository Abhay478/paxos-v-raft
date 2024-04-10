#![allow(dead_code)]
use std::{
    net::UdpSocket,
    // sync::{Arc, Mutex},
};

use itertools::Itertools;
use serde_json::{from_slice, to_vec};

use crate::paxos::{Ballot, Message, Proposal};

// type AcceptList = Arc<Mutex<Vec<Proposal>>>;

/// Acceptor struct.
struct Acceptor {
    /// Just a lil number. Unique among all acceptors.
    pub id: usize,
    // pub ballot: Arc<Mutex<Ballot>>,

    /// The current ballot number of the acceptor. Important thing.
    pub ballot: Ballot,

    /// All the stuff so far. 
    /// TODO: Should this be a map?
    pub accepted: Vec<Proposal>,
    
    /// This is us.
    pub sock: UdpSocket,
    buf: Vec<u8>,
}

impl Acceptor {
    pub fn new(id: usize, sock: UdpSocket) -> Acceptor {
        Acceptor {
            id,
            ballot: Ballot::new(0, 0),
            accepted: vec![],
            sock,
            buf: vec![],
        }
    }

    fn get_latest_accepts(&self) -> Vec<Proposal> {
        self.accepted
            .iter()
            .max_set()
            .into_iter()
            .cloned()
            .collect()
    }

    /// Promise
    fn receive_p1(&mut self, ballot: Ballot) -> Message {
        // Just do it.
        if ballot > self.ballot {
            self.ballot = ballot;
        }

        // Send that damnation message.
        Message::Phase1b(
            ballot.leader_id,
            self.id,
            self.ballot,
            self.get_latest_accepts(),
        )
    }

    /// Accept
    fn receive_p2(&mut self, leader_id: usize, proposal: Proposal) -> Message {
        if proposal.ballot == self.ballot {
            self.accepted.push(proposal.clone());
        }
        Message::Phase2b(leader_id, self.id, proposal.ballot)
    }

    /// Mux
    fn handle(&mut self, req: Message) -> Message {
        match req {
            Message::Phase1a(_num, ballot) => self.receive_p1(ballot),
            Message::Phase2a(lid, prop) => self.receive_p2(lid, prop),
            _ => unreachable!(),
        }
    }
}

/// This is the main loop for the acceptor. 
/// Acceptors are pretty dumb, so there's not much going on here.
pub fn listen(id: usize, sock: UdpSocket) {
    let mut q = Acceptor::new(id, sock);
    // let mut log = vec![];
    loop {
        let mut b = vec![];
        let (l, src) = q.sock.recv_from(&mut b).unwrap();

        let res = if let Ok(req) = from_slice(&b[..l]) {
            to_vec(&q.handle(req)).unwrap()
        } else {
            "Invalid message.".as_bytes().to_vec()
        };
        q.sock.send_to(&res, src).unwrap();
        // log.push(buf);
    }
}
