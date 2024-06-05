Coin - A simple cryptocurrency implementation
---------------------------------------------

This repo contains the implementation of a simple cryptocurrency blockchain which uses Proof of Work for block discovery, inspired by the Bitcoin protocol, and written in Rust.

All the components of this project are kept intentionally simple and only basic optimization strategies are used (namely an UTXO pool). The project rely on a single centralized node server and no decentralized protocol is implemented.

The project was born for educational purposes, the main goal was to build a basic functional cryptocurrency in few lines of code.


### Building

Usual `cargo` commands can be used, otherwise a simple Makefile is supplied.
Run `cargo test` or `make test` for running the test suite.
Run `cargo build` or `make build` for building the project.


### Running

The `client` and `server` CLIs show some documentation on their commands and how to run them.
