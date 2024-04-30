use std::{fs::File, io::Read, thread, time::Duration};

use rand::{
    distributions::{Distribution, Uniform},
    rngs::ThreadRng,
};
use serde::{Deserialize, Serialize};

pub const LOOPBACK: [u8; 4] = [127, 0, 0, 1];

pub mod paxos;
pub mod raft;

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub k: usize,
    l: f64,
}

impl Params {
    pub fn new() -> Self {
        let mut file = File::open("inp-params.txt").unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        let q = buf
            .split_whitespace()
            .map(|x| x.parse::<f64>().unwrap())
            .collect::<Vec<f64>>();

        Self {
            k: q[0] as usize,
            l: q[1],
        }
    }

    pub fn get_delay(u: Uniform<f64>, rng: &mut ThreadRng, l: f64) -> Duration {
        let ts = -(u.sample(rng) as f64).ln() * l;
        Duration::from_millis(ts as u64)
    }

    pub fn sleep(&self, u: Uniform<f64>, rng: &mut ThreadRng) {
        thread::sleep(Self::get_delay(u, rng, self.l));
    }
}

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
