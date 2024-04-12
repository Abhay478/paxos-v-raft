use std::{fs::File, io::Read, thread, time::Duration};

use rand::{
    distributions::{Distribution, Uniform},
    rngs::ThreadRng,
};

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

    pub fn sleep(&self, u: Uniform<f64>, rng: &mut ThreadRng) {
        let ts = -(u.sample(rng) as f64).ln() * self.l;
        thread::sleep(Duration::from_millis(ts as u64));
    }
}
