# Contributing to arkcfg

Thanks for helping improve arkcfg! This is a small, dependency-light Rust CLI, so
the workflow is intentionally short.

## Requirements

- A recent stable Rust toolchain (`rustup toolchain install stable`) with the
  `rustfmt` and `clippy` components.
- An X11 session if you want to exercise the display commands against real
  hardware (`xrandr` must be installed).

## Before opening a PR

Run the same checks CI runs:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

## Ground rules

- **Do not add dependencies casually.** arkanaOS builds this crate with
  `cargo build --locked --release` from the `main` tarball
  (`arkanas/03_arkpkg.mk` in [arkanaOS](https://github.com/arkana-team/arkanaOS)).
  Any dependency change must include an updated `Cargo.lock`, or every OS image
  build breaks. Prefer the standard library.
- **Commands that need root/sudo must refuse to run as root.** arkcfg never
  escalates privileges itself; it tells the user to re-run under `sudo`.
- **Never shell out through a shell string.** Build commands as
  `Command::new(program).args([...])` so display names and user input can never
  be interpreted by a shell.
- **Add a test for parser changes.** The `xrandr` output parser in
  `src/display.rs` is the most bug-prone part of the code; add a sample of real
  `xrandr` output to the tests with it.
- Keep user-facing errors actionable (say what the user can do next).

## Project layout

| File | Purpose |
| --- | --- |
| `src/main.rs` | Entry point, panic hook, error reporting, exit codes |
| `src/cli.rs` | `clap` command/flag definitions |
| `src/display.rs` | `xrandr` parsing and display commands |
| `src/power.rs` | reboot/poweroff with fallbacks |
| `src/sysinfo.rs` | fastfetch/neofetch/uname system information |
| `src/errors.rs` | Error type and user-facing messages |
