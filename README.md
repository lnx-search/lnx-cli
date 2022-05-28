# lnx-cli

A tool for demoing lnx or running benchmarks and load tests on Meilisearch or lnx itself.
This is generally use during development to help us profile and visually see any changes being ran into.

## Installing

The easiest way to install is via `cargo`

```
cargo install lnxcli --git https://github.com/lnx-search/lnx-cli.git
```

## Getting started

There are 2 sub commands `demo` and `bench` each with their own `--help` flags respectively which I wouold highly recommend reading.

**Demo** starts a webserver and web page linked to a given LNX instance where you can load a dataset or use the inbuilt movies dataset. This lets you change between query kinds 
and observe the results.

**Bench** allows you to benchmark both MeiliSearch and lnx respectively. This has two main modes `standard` or `typing` which changes if the system sends the full query instantly o
or types it out letter by letter emulating a user doing search as you type.
