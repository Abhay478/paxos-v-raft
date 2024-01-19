use std::{
    collections::HashMap,
    future::Future,
    net::SocketAddr,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread,
};

use tokio::net::UdpSocket;

use serde_json::to_vec;

use super::{Ballot, Message, Proposal};

pub enum Agent {
    Committed,
    Adopted(Ballot, HashMap<usize, Vec<Proposal>>),
    Preempted(Ballot),
}

impl Agent {
    /// Call this in a separate thread
    pub async fn init_commander(
        prop: Proposal,
        acceptors: Arc<Vec<SocketAddr>>,
        replicas: Arc<Vec<SocketAddr>>,
        sock: Arc<UdpSocket>,
        agent_tx: Arc<Sender<Agent>>,
        lid: usize,
    ) {
        let mut waitfor = (*acceptors).clone();
        let msg = Message::Phase2a(lid, prop.clone());
        // acceptors.iter().for_each( |acc| {tokio::spawn(async {
        //     sock.send_to(&to_vec(&msg).unwrap(), acc).await.unwrap();
        // });});
        // FFS

        for acc in acceptors.iter() {
            sock.send_to(&to_vec(&msg).unwrap(), acc).await.unwrap();
        }

        loop {
            let mut buf = vec![0; 1024];
            let (len, addr) = sock.recv_from(&mut buf).await.unwrap();
            let msg: Message = serde_json::from_slice(&buf[..len]).unwrap();
            match msg {
                Message::Phase2b(_back_lid, _acc_id, blt) => {
                    if blt == prop.ballot {
                        waitfor.retain(|x| *x != addr);

                        if waitfor.len() < acceptors.len() / 2 {
                            // Majority
                            let rep_msg = Message::Decision(prop.slot, prop.command.clone());

                            for rep in replicas.iter() {
                                sock.send_to(&to_vec(&rep_msg).unwrap(), rep).await.unwrap();
                            }
                            // return Self::Committed;
                            agent_tx.send(Self::Committed).unwrap();
                            return;
                        }
                    } else {
                        agent_tx.send(Self::Preempted(blt)).unwrap();
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    pub async fn init_scout(
        lid: usize,
        ballot_rx: Receiver<Ballot>,
        agent_tx: Arc<Sender<Agent>>,
        acceptors: Arc<Vec<SocketAddr>>,
        sock: Arc<UdpSocket>,
    ) {
        loop {
            let ballot = ballot_rx.recv().unwrap();
            let mut waitfor = (*acceptors).clone();
            let msg = Message::Phase1a(lid, ballot);

            for acc in acceptors.iter() {
                sock.send_to(&to_vec(&msg).unwrap(), acc).await.unwrap();
            }

            let mut pvals = HashMap::<usize, Vec<Proposal>>::new();

            loop {
                let mut buf = vec![0; 1024];
                let (len, addr) = sock.recv_from(&mut buf).await.unwrap();
                let msg: Message = serde_json::from_slice(&buf[..len]).unwrap();
                match msg {
                    Message::Phase1b(_lid, _acc_id, blt, accepts) => {
                        if blt == ballot {
                            waitfor.retain(|x| *x != addr);
                            accepts.iter().for_each(|acc| {
                                if let Some(p) = pvals.get_mut(&acc.slot) {
                                    p.push(acc.clone());
                                } else {
                                    pvals.insert(acc.slot, vec![acc.clone()]);
                                }
                            });

                            if waitfor.len() < acceptors.len() / 2 {
                                // Majority
                                agent_tx.send(Self::Adopted(blt, pvals)).unwrap();
                                break;
                            }
                        } else {
                            agent_tx.send(Self::Preempted(blt)).unwrap();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub struct Leader {
    id: usize,
    proposals: HashMap<usize, Proposal>,
    active: bool,
    ballot: Ballot,
}

impl Leader {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            proposals: HashMap::new(),
            active: false,
            ballot: Ballot::new(id, 0),
        }
    }

    pub fn update(&mut self, pmax: HashMap<usize, Proposal>) {
        self.proposals.retain(|s, p| match pmax.get(&s) {
            Some(val) => val.command == p.command,
            None => true,
        });

        self.proposals.extend(pmax.into_iter());
    }
}

pub fn get_pmax(pvals: &HashMap<usize, Vec<Proposal>>) -> HashMap<usize, Proposal> {
    let pmax = pvals
        .into_iter()
        .map(|(slot, prop)| {
            (
                *slot,
                prop.into_iter()
                    .max_by_key(|p| p.ballot.num)
                    .unwrap()
                    .clone(),
            )
        })
        .collect::<HashMap<usize, Proposal>>();
    pmax
}

fn async_block(
    prop: Proposal,
    acceptors: Arc<Vec<SocketAddr>>,
    replicas: Arc<Vec<SocketAddr>>,
    sock: Arc<UdpSocket>,
    agent_tx: Arc<Sender<Agent>>,
    lid: usize,
) -> impl Future<Output = ()> {
    async move { Agent::init_commander(prop, acceptors, replicas, sock, agent_tx, lid).await }
}

/// TODO: Add file read for lists.
pub async fn listen(id: usize, sock: UdpSocket) {
    let arc_sock = Arc::new(sock);

    let acceptors = Arc::new(vec![]);
    let replicas = Arc::new(vec![]);

    let mut leader = Leader::new(id);

    let (ballot_tx, ballot_rx) = channel::<Ballot>();
    let (agent_tx, agent_rx) = channel::<Agent>();

    let agent_tx = Arc::new(agent_tx);

    let mut commanders = vec![];

    let alt_sock = arc_sock.clone();
    let new_acc = acceptors.clone();
    let new_tx = agent_tx.clone();

    let _scout = thread::spawn(move || async move {
        Agent::init_scout(leader.id, ballot_rx, new_tx, new_acc, alt_sock).await;
    }); // Sus

    loop {
        if let Ok(out) = agent_rx.try_recv() {
            match out {
                Agent::Adopted(blt, pvals) => {
                    leader.ballot.num = blt.num + 1;
                    let pmax = get_pmax(&pvals);
                    leader.update(pmax);

                    // This is bad. Too many clones.
                    for (_s, p) in leader.proposals.iter() {
                        let alt_sock = arc_sock.clone();
                        let new_acc = acceptors.clone();
                        let new_rep = replicas.clone();
                        let new_tx = agent_tx.clone();
                        let q = p.clone();

                        commanders.push(thread::spawn(move || {
                            async_block(q, new_acc, new_rep, alt_sock, new_tx, leader.id)
                        }));
                    }

                    leader.active = true;
                }
                Agent::Preempted(blt) => {
                    if blt > leader.ballot {
                        leader.active = false;
                        leader.ballot.num = blt.num + 1;
                        ballot_tx.send(leader.ballot).unwrap();
                    } 
                }
                Agent::Committed => {}
            }
        }

        let mut buf = vec![0; 1024];
        if let Ok((len, _addr)) = arc_sock.recv_from(&mut buf).await {
            let alt_sock = arc_sock.clone(); // We need a new one.
            let msg: Message = serde_json::from_slice(&buf[..len]).unwrap();

            match msg {
                Message::Propose(slot, cmd) => {
                    if let Some(_) = leader.proposals.get(&slot) {
                        continue;
                    }

                    let prop = Proposal {
                        slot,
                        ballot: leader.ballot,
                        command: cmd,
                    };
                    leader.proposals.insert(slot, prop.clone());

                    let new_acc = acceptors.clone();
                    let new_rep = replicas.clone();
                    let new_tx = agent_tx.clone();
                    if leader.active {
                        commanders.push(thread::spawn(move || {
                            async_block(prop, new_acc, new_rep, alt_sock, new_tx, leader.id)
                        }))
                    }
                }
                Message::Terminate => {
                    break;
                }
                _ => {}
            }
        }
    }
    // todo!()
}
