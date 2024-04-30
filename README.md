# Paxos vs Raft


## Project File Structure

- src: All rust files
  - bin: Binaries independent of the library
    - `acceptor.rs`: Acceptors for Paxos
    - `client.rs`: Client for Paxos
    - `leader.rs`: Leader for Paxos
    - `replica.rs`: Replica for Paxos
    - `raft.rs`: Server for Raft
    - `raft_client.rs`: Client for Raft
    - `paxos_threads.rs`: Threads for Paxos
    - `raft_threads.rs`: Threads for Raft
  - paxos: Paxos implementation
  - raft: Raft implementation
  - lib.rs: Module root
- `inp-params.txt`: Input parameters for the algorithms. At the current stage, it is of the form "k l", where k is the number of requests made by the client, and l is the parameter for the exponential distribution from which the sleep time between requests is sampled.
- `README.md`: This file
- `report.md`, `report.pdf`: TODO

# Execution Instructions

- As of 2032, 30/4/2024, the algorithms have not been tested on multiple machines due to lack of time. The library assures that only minor changes to the socket binding addresses are necessary to run the distributed algorithms.
- The correctness of the algorithmsis verified through the multithreaded implementation, in `src/bin/paxos_threads.rs` and `src/bin/raft_threads.rs`. Refer those files for commands to run.