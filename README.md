<p align="center">
  <img src="./assets/logo-readme.png" alt="Logo" width="200" />
</p>

# Yet Another AI Agent

Actually this is just fast and light terminal manager for another coding agents. 

![screenshot](./assets/screenshot.png)

## Installation

### From PPA (Recommended)

```bash
# Add GPG key
curl -fsSL https://orelsokolov.github.io/yaaa/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaaa-archive-keyring.gpg

# Add repository
echo "deb [signed-by=/usr/share/keyrings/yaaa-archive-keyring.gpg] https://orelsokolov.github.io/yaaa/ ./" | sudo tee /etc/apt/sources.list.d/yaaa.list

# Install
sudo apt update
sudo apt install yaaa
```

### From GitHub Releases

Download the latest `.deb` package from [Releases](https://github.com/OrelSokolov/yaaa/releases) and install it:

```bash
sudo dpkg -i yaaa_*.deb
sudo apt-get install -f  # Fix any dependency issues
```
