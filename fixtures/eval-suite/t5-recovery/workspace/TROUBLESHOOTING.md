# Troubleshooting

## E0425: cannot find function `normalize_port`

The helper was renamed during a previous cleanup. Do not add a dependency and
do not change the toolchain. Import or call `port::normalize` from `src/port.rs`
and keep the public `listen_address` behavior unchanged.
