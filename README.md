<p align="center">
  <img src="./assets/logo-readme.png" alt="Logo" width="200" />
</p>

# Yet Another AI Agent

Actually this is just fast and light terminal manager for another coding agents. 

![screenshot](./assets/screenshot.png)

## Installation from PPA

Add the PPA repository:

```bash
echo "deb [trusted=yes] https://orelsokolov.github.io/yaaa stable main" | sudo tee /etc/apt/sources.list.d/yaaa.list
```

Update package list and install:

```bash
sudo apt update
sudo apt install yaaa
```

## Building from source

```bash
cargo build --release
```
