use std::{net::{SocketAddr, UdpSocket}, thread::JoinHandle, sync::{Mutex, Arc}};

use serde_json::to_vec;

use super::{Proposal, Message, Ballot, Command};

pub enum Agent {
    Committed,
    Adopted(Ballot, Vec<Proposal>),
    Preempted(Ballot),
}

impl Agent {
    /// Call this in a separate thread
    pub fn init_commander(
        prop: Proposal,
        acceptors: &Vec<SocketAddr>,
        replicas: Vec<SocketAddr>,
        sock: UdpSocket,
        lid: usize,
    ) -> Self {
        let mut waitfor = acceptors.clone();
        // let b = prop.ballot;
        // let c = prop.command.clone();
        let msg = Message::Phase2a(lid, prop.clone());
        acceptors.iter().for_each(|acc| {
            sock.send_to(&to_vec(&msg).unwrap(), acc).unwrap();
        });

        loop {
            let mut buf = vec![0; 1024];
            let (len, addr) = sock.recv_from(&mut buf).unwrap();
            let msg: Message = serde_json::from_slice(&buf[..len]).unwrap();
            match msg {
                Message::Phase2b(_back_lid, _acc_id, blt) => {
                    // waitfor.remove(waitfor.iter().position(|a| a == &addr).unwrap());
                    if blt == prop.ballot {
                        waitfor.retain(|x| *x != addr);

                        if waitfor.len() < acceptors.len() / 2 { // Majority
                            let rep_msg = Message::Decision(prop.slot, prop.command);
                            replicas.iter().for_each(|rep| {
                                sock.send_to(&to_vec(&rep_msg).unwrap(), rep).unwrap();
                            });
                            return Self::Committed;
                        }
                    }
                    else {
                        return Self::Preempted(blt);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn init_scout(
        lid: usize,
        ballot: Ballot,
        acceptors: &Vec<SocketAddr>,
        sock: UdpSocket,
    ) -> Self {
        let mut waitfor = acceptors.clone();
        let msg = Message::Phase1a(lid, ballot);
        acceptors.iter().for_each(|acc| {
            sock.send_to(&to_vec(&msg).unwrap(), acc).unwrap();
        });

        let mut pvals = vec![];

        loop {
            let mut buf = vec![0; 1024];
            let (len, addr) = sock.recv_from(&mut buf).unwrap();
            let msg: Message = serde_json::from_slice(&buf[..len]).unwrap();
            match msg {
                Message::Phase1b(_lid, _acc_id, blt, mut accepts) => {
                    // waitfor.remove(waitfor.iter().position(|a| a == &addr).unwrap());
                    if blt == ballot {
                        waitfor.retain(|x| *x != addr);
                        pvals.append(&mut accepts);

                        if waitfor.len() < acceptors.len() / 2 { // Majority
                            return Self::Adopted(blt, pvals);
                        }
                    }
                    else {
                        return Self::Preempted(blt);
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct Leader {
    pub id: usize,
    pub active: bool,
    pub ballot_num: usize,
    pub proposals: Vec<Command>,
    pub scout: Option<JoinHandle<Agent>>,
    pub sock: UdpSocket,
    pub commanders: Vec<JoinHandle<Agent>>,
    pub acceptors: Vec<SocketAddr>,
    pub replicas: Vec<SocketAddr>,
    pub lock: Arc<Mutex<()>>,
}

impl Leader {
    pub fn new(
        id: usize,
        acceptors: Vec<SocketAddr>,
        replicas: Vec<SocketAddr>,
        sock: UdpSocket,
    ) -> Self {
        let lock = Arc::new(Mutex::new(()));
        // let scout = std::thread::spawn(move || {
        //     let ballot = Ballot::new(id, 0);
        //     Agent::init_scout(id, ballot, &acceptors, sock)
        // });
        Self {
            id,
            active: false,
            ballot_num: 0,
            proposals: vec![],
            scout: None,
            sock,
            commanders: vec![],
            acceptors,
            replicas,
            lock,
        }
    }
}

fn listen(id: usize, sock: UdpSocket) {
    todo!()
}