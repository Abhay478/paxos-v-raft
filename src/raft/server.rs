#![allow(dead_code)]
use std::{net::SocketAddr, time::Duration};

use hashbrown::HashMap;
use message_io::{
    events::TimerId,
    network::{Endpoint, NetEvent, Transport},
    node::{self, NodeEvent, NodeHandler},
};
use rand::distributions::{Distribution, Uniform};
use serde_json::{from_slice, to_vec};

use crate::ReplicaState;

use super::{dir::get_peers, Campaign, Heartbeat, Log, Message, Replicate, Reply, ServerState, Timer};


pub struct Server {
    id: usize,
    state: ServerState,                 // Look at enum variants
    rst: ReplicaState,                  // State of the replica
    current_term: usize,                // Rounds numbers
    voted_for: Option<usize>,           // Leader election.
    log: Vec<Log>,                      // Replica<index, Log<term, ACTUAL SHIT>>
    commit_index: usize,                // index of highest committed entry
    last_applied: usize,                // index of highest applied entry
    next_index: HashMap<usize, usize>,  // index of next log entry to send to each server
    match_index: HashMap<usize, usize>, // index of highest log entry known to be replicated on server

    u: rand::distributions::Uniform<f64>,
    handler: NodeHandler<Timer>,
    peers: HashMap<usize, Endpoint>,
    clients: HashMap<SocketAddr, Endpoint>,
    current_timer: Option<TimerId>,
}

impl Server {
    fn new(id: usize, peers: HashMap<usize, Endpoint>, handler: NodeHandler<Timer>) -> Self {
        let mut out = Self {
            id,
            state: ServerState::Follower,
            rst: ReplicaState::default(),
            current_term: 0,
            voted_for: None,
            log: vec![],
            commit_index: 0,
            last_applied: 0,
            next_index: HashMap::new(),
            match_index: HashMap::new(),
            u: Uniform::new(150.0, 300.0),
            handler,
            peers,
            clients: HashMap::new(),
            current_timer: None,
        };

        // Start the timeouts.
        out.reset_timeout();
        // out.reset_heartbeat();
        out
    }

    /*  /// Heartbeat with no entries
    fn empty_decree(&self) {
        if self.state == ServerState::Leader {
            let hb = Heartbeat {
                term: self.current_term,
                leader_id: self.id,
                prev_log_index: 0,
                prev_log_term: 0,
                leader_commit: self.commit_index,
                entries: vec![],
            };

            for p in self.peers.iter() {
                self.handler.network().send(*p, &to_vec(&hb).unwrap());
            }

            self.reset_heatbeat();
        }
    }

    fn decree(&self, pli: usize) {
        if self.state == ServerState::Leader {
            let entries = self
                .log
                .iter()
                .filter(|x| *x.0 > pli)
                .map(|x| (*x.0, x.1.clone()))
                .collect::<Vec<_>>();

            let hb = Heartbeat {
                term: self.current_term,
                leader_id: self.id,
                prev_log_index: pli,
                prev_log_term: self.log[&pli].term,
                leader_commit: self.commit_index,
                entries,
            };

            for p in self.peers.iter() {
                self.handler.network().send(*p, &to_vec(&hb).unwrap());
            }

            self.reset_heatbeat();
        }
    } */

    /* fn get_decree_entries(&self, follower_id: usize) -> (usize, Vec<(usize, Log)>) {
        let pli = self.next_index[&follower_id] - 1;
        (pli, self
            .log
            .iter()
            .filter(|x| *x.0 > pli)
            .map(|x| (*x.0, x.1.clone()))
            .collect::<Vec<_>>())
    }

    fn get_heartbeat_metadata(&self) -> Heartbeat {

    }
    */
    fn empty_decree(&self) {
        let hb = Heartbeat {
            term: self.current_term,
            leader_id: self.id,
            prev_log_index: self.log.len() - 1,
            prev_log_term: self.log.last().unwrap().term,
            leader_commit: self.commit_index,
        };

        for p in self.peers.iter() {
            self.handler
                .network()
                .send(*p.1, &to_vec(&Message::Heartbeat(Replicate { hb, entries: vec![] })).unwrap());
        }
    }

    fn decree(&self) {
        let hb = Heartbeat {
            term: self.current_term,
            leader_id: self.id,
            prev_log_index: self.log.len() - 1,
            prev_log_term: self.log.last().unwrap().term,
            leader_commit: self.commit_index,
        };

        for p in self.peers.iter() {
            let entries = self.log[self.next_index[p.0]..self.log.len()]
                .iter()
                .enumerate()
                .map(|x| (x.0, x.1.clone()))
                .collect::<Vec<_>>();
            self.handler
                .network()
                .send(*p.1, &to_vec(&Message::Heartbeat(Replicate { hb, entries })).unwrap());
        }
    }

    fn campaign(&mut self) {
        self.reset_timeout();
        self.current_term += 1;
        self.voted_for = Some(self.id);
        self.state = ServerState::Candidate(1);

        let msg = Campaign {
            term: self.current_term,
            candidate_id: self.id,
            last_log_index: self.log.len() - 1,
            last_log_term: self.log.last().unwrap().term,
        };

        for p in self.peers.iter() {
            self.handler.network().send(*p.1, &to_vec(&msg).unwrap());
        }
    }

    fn crown(&mut self) {
        self.state = ServerState::Leader;
        self.voted_for = None;
        self.empty_decree();
        for (_a, b) in self.next_index.iter_mut() {
            *b = self.log.len();
        }
    }

    fn reset_timeout(&mut self) {
        let rng = &mut rand::thread_rng();
        if let Some(t) = self.current_timer {
            self.handler.signals().cancel_timer(t);
        }
        self.current_timer = Some(self.handler.signals().send_with_timer(
            Timer::Election,
            Duration::from_millis(self.u.sample(rng) as u64),
        ));
    }

    fn reject(&self, ep: Endpoint) {
        let rep = &Message::ServerReply(Reply {
            from: self.id,
            success: false,
            term: self.current_term,
        });
        self.handler.network().send(ep, &to_vec(rep).unwrap());
    }

    fn vote(&mut self, ep: Endpoint, c: Campaign) {
        self.current_term = c.term;
        self.voted_for = Some(c.candidate_id);
        self.state = ServerState::Follower;
        let rep = &Message::ServerReply(Reply {
            from: self.id,
            success: true,
            term: self.current_term,
        });
        self.handler.network().send(ep, &to_vec(rep).unwrap());
        self.reset_timeout();
    }

    fn accept(&self, ep: Endpoint) {
        let rep = &Message::ServerReply(Reply {
            from: self.id,
            success: true,
            term: self.current_term,
        });
        self.handler.network().send(ep, &to_vec(rep).unwrap());
    }

    fn perform(&mut self) {
        for q in self.last_applied + 1..=self.commit_index {
            // perform
            let cmd = self.log[q].command.clone();
            let (state, _res) = ReplicaState::triv(cmd.op.clone())(&self.rst);
            self.rst = state;
            if self.state == ServerState::Leader {
                let sock = self.log[q].command.client;
                if let Some(ep) = self.clients.get(&sock) {
                    self.handler
                        .network()
                        .send(*ep, &to_vec(&Message::Response(cmd.clone())).unwrap());
                } else {
                    let ep = self
                        .handler
                        .network()
                        .connect(Transport::Udp, sock)
                        .unwrap()
                        .0;
                    self.clients.insert(sock, ep);
                    self.handler
                        .network()
                        .send(ep, &to_vec(&Message::Response(cmd.clone())).unwrap());
                };
            }
        }
        self.last_applied = self.commit_index;
    }
}

pub fn run(id: usize, addr: SocketAddr) {
    let (handler, listener) = node::split::<Timer>();
    handler.network().listen(Transport::Udp, addr).unwrap();
    let peers = get_peers(id, handler.clone());

    let mut server = Server::new(id, peers, handler);
    let _ = listener.for_each_async(move |event| {
        match event {
            NodeEvent::Network(e) => {
                match e {
                    NetEvent::Connected(ep, _) => {
                        println!("Raft server {id} Connected to {ep}.");
                    }
                    NetEvent::Accepted(ep, _) => {
                        println!("Raft server {id} Accepted {ep}.");
                    }
                    NetEvent::Disconnected(ep) => {
                        println!("Raft server {id} Disconnected from {ep}.");
                    }
                    NetEvent::Message(ep, buf) => {
                        let msg = from_slice::<Message>(&buf).unwrap();
                        match msg {
                            // If leader, decree. Else, redirect to leader.
                            Message::Request(cmd) => {
                                if server.state == ServerState::Leader {
                                    server.log.push(Log {
                                        term: server.current_term,
                                        command: cmd,
                                    });
                                    server.decree();
                                } else {
                                    // forward to leader
                                    let leader = server.peers[&server.voted_for.unwrap()];
                                    server.handler.network().send(leader, &buf);
                                }
                            }

                            // A server can never receive a response.
                            // The leader responds to the client directly.
                            // The client socket address is contained in the command.
                            Message::Response(_) => unreachable!(),
                            // Add to log
                            Message::Heartbeat(rep) => {
                                if rep.hb.term < server.current_term // Old leader
                                    || server.log.len() - 1 < rep.hb.prev_log_index // Old log, send previous stuff also
                                    || server.log[rep.hb.prev_log_index].term // Log conflict, send previous stuff also
                                        != rep.hb.prev_log_term
                                {
                                    server.reject(ep);
                                } else {
                                    // Assuming the entries are in order.
                                    for (i, l) in rep.entries.iter() {
                                        if *i < server.log.len() {
                                            // Override
                                            server.log[*i] = l.clone();
                                        } else {
                                            // New
                                            server.log.push(l.clone());
                                        }
                                    }
                                }

                                if rep.hb.leader_commit > server.commit_index {
                                    server.commit_index =
                                        rep.hb.leader_commit.min(server.log.len() - 1);
                                    if server.commit_index > server.last_applied {
                                        // perform
                                        server.perform();
                                    }
                                }

                                server.accept(ep);
                                server.reset_timeout();
                            }
                            // Candidacy
                            Message::Campaign(c) => {
                                // Somehow, we don't need to check ServerState?

                                // If we at newer term, reply false.
                                if c.term < server.current_term {
                                    server.reject(ep);
                                }
                                // Ambiguity here. Should we abstain?
                                else if c.term > server.current_term
                                    || c.last_log_index >= server.log.len() - 1
                                {
                                    // Can be optimised, not doing it.
                                    if let Some(s) = server.voted_for {
                                        if s == c.candidate_id {
                                            server.vote(ep, c);
                                        }
                                    } else if server.voted_for.is_none() {
                                        server.vote(ep, c);
                                    }
                                }
                            }

                            Message::ServerReply(res) => {
                                match server.state {
                                    ServerState::Follower => {}
                                    // Votes
                                    ServerState::Candidate(v) => {
                                        if res.success {
                                            // Majority
                                            if v > (server.peers.len() + 1) / 2 {
                                                server.crown();
                                            } else {
                                                server.state = ServerState::Candidate(v + 1);
                                            }
                                        } else {
                                            server.current_term = res.term;
                                            server.state = ServerState::Follower;
                                            // server.campaign();
                                        }
                                    }
                                    // Rejects
                                    ServerState::Leader => {
                                        if res.term > server.current_term {
                                            server.current_term = res.term;
                                            server.state = ServerState::Follower;
                                            // todo!()
                                        } else {
                                            if res.success {
                                                // TODO: Verify.
                                                server
                                                    .match_index
                                                    .insert(res.from, server.log.len() - 1);
                                                server
                                                    .next_index
                                                    .insert(res.from, server.log.len());

                                                for i in (server.commit_index + 1..server.log.len())
                                                    .rev()
                                                {
                                                    let mut count = 1;
                                                    for (_a, b) in server.match_index.iter() {
                                                        if *b >= i {
                                                            count += 1;
                                                        }
                                                    }
                                                    if count > (server.peers.len() + 1) / 2 {
                                                        server.commit_index = i;
                                                        break;
                                                    }
                                                }

                                                if server.commit_index > server.last_applied {
                                                    server.perform();
                                                }
                                            } else {
                                                // Term matches, log does not.
                                                *server.next_index.get_mut(&res.from).unwrap() -= 1;
                                                server.decree();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            NodeEvent::Signal(t) => match t {
                Timer::Heartbeat => {
                    server.empty_decree();
                }
                Timer::Election => {
                    server.campaign();
                }
            },
        }
    });
}
