# Setting Up GitHub Actions Runner on Pi 5 ⚡️

PacketParamedic uses a self-hosted runner on the Raspberry Pi 5 to ensure tests run on the actual target hardware (ARM64, Cortex-A76, VideoCore VII), avoiding cross-compilation quirks.

## 1. Prerequisites
*   You must be an owner or collaborator on the GitHub repository: `ParkWardRR/PacketParamedic`.
*   The Pi 5 (`alfa@PacketParamedic.alpina`) must have internet access.

## 2. Obtain a Runner Token
1.  Navigate to your repository on GitHub.
2.  Go to **Settings** > **Actions** > **Runners**.
3.  Click **New self-hosted runner**.
4.  Select **Runner image**: `Linux`.
5.  Select **Architecture**: `ARM64`.
6.  Look for the token in the command block (e.g., `./config.sh --url ... --token THIS_IS_THE_TOKEN`). Copy just the token.

## 3. Run the Setup Script
SSH into your Pi 5 and run the provided setup tool:

```bash
# On your local machine (macOS/Linux):
rsync -avz ./tools alfa@PacketParamedic.alpina:~/PacketParamedic/

# On the Pi 5:
ssh alfa@PacketParamedic.alpina
cd ~/PacketParamedic
chmod +x tools/setup-runner.sh
./tools/setup-runner.sh <PASTE_YOUR_TOKEN_HERE>
```

The script will:
1.  Download the latest GitHub Actions Runner for Linux ARM64.
2.  Install dependencies (.NET Core runtime, libssl, etc).
3.  Configure the runner with labels: `self-hosted`, `linux`, `arm64`, `pi5`.
4.  Install and start the runner as a systemd service.

## 4. Verification
Check the status of the service:
```bash
sudo systemctl status actions.runner.*
```
You should see `Active: active (running)`.

The workflow `.github/workflows/pi5-ci.yml` will now automatically pick up this runner for any push to `main` or pull request.
