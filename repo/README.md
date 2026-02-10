# Yaaa PPA Repository

This directory contains the Debian/Ubuntu package repository for yaaa.

## Installation

Add the repository to your system:

```bash
echo "deb [trusted=yes] https://OrelSokolov.github.io/yaaa stable main" | sudo tee /etc/apt/sources.list.d/yaaa.list
```

Update and install:

```bash
sudo apt update
sudo apt install yaaa
```

## Repository Structure

- `dists/stable/` - Repository metadata
- `pool/stable/main/` - Package files
