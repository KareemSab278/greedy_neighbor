# Greedy Nearest Neighbor Algorithm

This repository contains a Rust implementation of a greedy nearest neighbor algorithm.

## Overview

The program reads data, computes a path using the greedy nearest neighbor strategy, and writes the result to the configured output. It can be run directly or managed as a systemd service.

## Running Locally

Build the project:

```bash
cargo build --release
```

Run the binary:

```bash
./target/release/greedy-nearest-neighbor-algorithm
```

## Systemd Service Setup

If you see an error indicating the service file wasn’t created or is missing, follow these steps.

### 1. Check it exists

```bash
ls /etc/systemd/system/greedy-nn.service
```

If it says `No such file`, that’s the issue.

---

### 2. Create it properly

```bash
sudo nano /etc/systemd/system/greedy-nn.service
```

Paste this:

```ini
[Unit]
Description=Greedy Nearest Neighbor Algorithm
After=network.target

[Service]
WorkingDirectory=/home/admin01/greedy_neighbor
ExecStart=/home/admin01/greedy_neighbor/target/release/greedy-nearest-neighbor-algorithm
Restart=always
RestartSec=2
User=admin01

[Install]
WantedBy=multi-user.target
```

Save and exit.

---

### 3. Reload systemd (important)

```bash
sudo systemctl daemon-reload
```

---

### 4. Enable + start

```bash
sudo systemctl enable greedy-nn
sudo systemctl start greedy-nn
```

---

### 5. If it still fails, check logs

```bash
journalctl -u greedy-nn -f
```

## Notes

- Update the paths in the service file if your repository is located somewhere other than `/home/admin01/greedy_neighbor`.
- Make sure the binary exists at `target/release/greedy-nearest-neighbor-algorithm` before starting the service.
