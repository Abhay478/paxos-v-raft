---
title: Comparison of State Replication Algorithms (Paxos \& Raft)
author: 
- "Abhay Shankar K: cs21btech11001"
- "Deepshikha: cs21btech11016"
geometry: "margin=2cm"
---

# Introduction

The aim of a state replication algorithm is to produce replicas of the same state in a designated set of nodes, named Replicas. A vast variety of algorithms exist, but have been shown to be equivalent to one of two main algorithms: Paxos and Raft. This document aims to compare the two algorithms, and highlight their differences and similarities.

# Paxos

Paxos is a family of protocols for solving consensus in a network of unreliable processors. It was first described by Leslie Lamport in 1989. The protocol is based on a consensus algorithm, known as the *Synod algorithm* or *single decree paxos* which is used to agree on a single data value among distributed processes or systems. Paxos uses this algorithm as a subroutine, and often, "Paxos" is simply used to refer to this subroutine.

It is a fundamental part of many distributed systems and is used to ensure that all nodes in a cluster agree on the same sequence of commands to execute.

## Actors

*Multi-decree Paxos* has three main actors:

- **Replicas**: Nodes responsible for maintaining the state. Each replica contains a log, and the objective of the algorithm is to ensure all replicas hold identical logs. A client may request a replica to execute a command, and the replica will propose the command.
- **Leaders**: A set of nodes that coordinate the replication of state across the cluster. The leader is responsible for proposing new commands to be added to the log.
- **Acceptors**: These nodes may accept (or reject) proposals by the leaders. The interaction between leaders and acceptors is the core of the Paxos algorithm.

# Raft

Raft is a consensus algorithm designed as an alternative to Paxos, designed by Diego Ongaro and John Ousterhout. Their main goal was it's understandability, due to the notorious opacity of Paxos.

It is based on the concept of a leader, which is elected by the nodes in the cluster. It does not have an analogue to the *single-decree paxos* subroutine, which the creators claim is one of the reasons why Raft is superior.

The leader is responsible for managing the replication of the state across the cluster. Raft is designed to be more understandable than Paxos, and is thus easier to implement and verify.
