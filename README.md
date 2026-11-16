<p align="center">
  <img src="./assets/logo.png" width="320" alt="ring">
</p>

<p align="center">
  <samp>Your internet speed, at a glance. Right in your terminal.</samp>
</p>

<p align="center">
  <a href="#features">Features</a> ·
  <a href="#install">Install</a> ·
  <a href="#usage">Usage</a>
</p>

<br>

<p align="center">
  <img src="./assets/demo.gif" width="800" alt="Ring running a live terminal speed test with latency, download and upload measurements">
</p>

<br>

A small internet speed tester built with Rust and [Ratatui](https://ratatui.rs). Open it, let it run, and see your connection's latency, download speed, and upload speed in one view.

## Features

- **Three measurements** — HTTP latency in ms, download and upload in Mbps.
- **Live results** — speed readings update as data transfers.
- **Terminal UI** — color-coded metrics and a download progress bar.
- **Keyboard first** — rerun a finished test or quit with a single key.

## Install

Requires [Rust and Cargo](https://rustup.rs).

```sh
cargo install --git https://github.com/chann44/ring --locked
```

Or, from a local checkout:

```sh
./install.sh
```

Make sure `~/.cargo/bin` (or `$CARGO_HOME/bin`) is on your `PATH`.

## Usage

```sh
ring
```

The test starts automatically using Cloudflare's speed test endpoints. Each run requests a 25 MB download and sends a 10 MB upload.

| Key | Action |
| --- | --- |
| `r` | Run again after completion or an error |
| `q` / `Esc` | Quit |

<details>
<summary>Record the demo</summary>

The GIF is recorded from the real app with [VHS](https://github.com/charmbracelet/vhs). With VHS, ttyd, and FFmpeg installed, run these commands from the repository root:

```sh
cargo build --release --locked
vhs assets/demo.tape
```

The recording uses a live connection, so timings and results will vary. The editable tape lives in [`assets/demo.tape`](./assets/demo.tape).

</details>

<br>

<p align="center">
  <sub>Made by <a href="https://github.com/chann44">chan</a></sub>
</p>
