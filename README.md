# RedisFSM

RedisFSM (fsm) is a [Redis](https://redis.io/) module that implement a toy Finite State Machine (FSM)
for Redis Hashes.
RefisFSM is built using [redismodule-rs](https://crates.io/crates/redis-module) an idiomatic Rust API
for the [Redis Modules API](https://redis.io/docs/reference/modules/).

## Build

Make sure you have Rust installed:
https://www.rust-lang.org/tools/install

Then, build as usual:

```bash
cargo build
```

Make sure you have Redis installed.

## Run

### Linux

```
redis-server --loadmodule ./target/debug/libredis_fsm.so
```

### Mac OS

```
redis-server --loadmodule ./target/debug/libredis_fsm.dylib
```

## License

MIT