# TODO

- [ ] Improve rtsp connection test/mock handling to support connection racing strategy with configurable parameters like max attempts, timeout, and backoff strategy.
- [ ] Feature-gate the use of `tokio` behind a Cargo feature flag to avoid unnecessary dependencies for users not needing async functionality.
- [ ] Feature gate the use of `dirs` crate behind a Cargo feature flag to avoid unnecessary dependencies for users not needing filesystem path handling.
- [ ] Feature gate the use of `rand` crate behind a Cargo feature flag to avoid unnecessary dependencies for users not needing randomization functionality.
- [ ] Cleanup and refactor the state change tests in `tests/rtspsrc.rs` to ensure clarity and correctness.
- [ ] Add more comprehensive integration tests with actual RTSP servers and various network conditions to validate the connection retry logic.
