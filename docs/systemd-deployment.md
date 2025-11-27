# Systemd Deployment Guide

This guide explains how to run the Discord Persona Bot as a systemd service, ensuring it runs persistently in the background and survives SSH disconnections and system reboots.

## Prerequisites

- The bot must be compiled in release mode: `cargo build --release`
- The `.env` file must be properly configured with all required environment variables
- The system must have systemd available (standard on most modern Linux distributions)

## Available Binaries

The project builds multiple binaries:
- **`bot`** - The main Discord bot (Gateway WebSocket connection) - **USE THIS ONE**
- **`persona`** - Simple test binary (just prints "Hello, world!")

The systemd service is configured to run the `bot` binary.

## Service Configuration

The `persona.service` file in the project root contains the systemd service definition:

```ini
[Unit]
Description=Discord Persona Bot
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=caavere
Group=caavere
WorkingDirectory=/home/caavere/Projects/bot/persona
ExecStart=/home/caavere/Projects/bot/persona/target/release/bot
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=persona-bot

# Load environment variables from .env file
EnvironmentFile=/home/caavere/Projects/bot/persona/.env

[Install]
WantedBy=multi-user.target
```

## Installation Steps

### Quick Setup with Makefile (Recommended)

The project includes a self-documented Makefile for easier management:

```bash
# View all available commands
make help

# Complete setup in one command
make setup

# Start the service
make start

# View logs in real-time
make logs-follow
```

### Manual Installation Steps

If you prefer to install manually:

1. **Build the release binary:**
   ```bash
   cargo build --release
   ```

2. **Copy the service file to systemd:**
   ```bash
   sudo cp /home/caavere/Projects/bot/persona/persona.service /etc/systemd/system/
   ```

3. **Reload systemd daemon:**
   ```bash
   sudo systemctl daemon-reload
   ```

4. **Enable the service to start on boot:**
   ```bash
   sudo systemctl enable persona.service
   ```

5. **Start the service:**
   ```bash
   sudo systemctl start persona.service
   ```

6. **Verify the service is running:**
   ```bash
   sudo systemctl status persona.service
   ```

## Service Management

### Using Makefile (Recommended)

```bash
# View service status
make status

# Start the service
make start

# Stop the service
make stop

# Restart the service
make restart

# Uninstall the service
make uninstall-service
```

### Using systemctl Directly

```bash
# View service status
sudo systemctl status persona.service

# Stop the bot
sudo systemctl stop persona.service

# Start the bot
sudo systemctl start persona.service

# Restart the bot
sudo systemctl restart persona.service

# Disable auto-start on boot
sudo systemctl disable persona.service

# Re-enable auto-start on boot
sudo systemctl enable persona.service
```

## Viewing Logs

The bot outputs to systemd's journal. You can view logs using the Makefile or `journalctl`:

### Using Makefile (Recommended)

```bash
# View real-time logs
make logs-follow

# View recent logs (last 100 lines)
make logs

# View logs since last boot
make logs-boot

# Export logs to a file
make logs-export
```

### Using journalctl Directly

```bash
# View real-time logs
sudo journalctl -u persona.service -f

# View recent logs (last 100 lines)
sudo journalctl -u persona.service -n 100

# View logs since boot
sudo journalctl -u persona.service -b

# View logs from a specific time range
sudo journalctl -u persona.service --since "2024-01-01 00:00:00" --until "2024-01-01 23:59:59"

# Export logs to a file
sudo journalctl -u persona.service > persona-bot.log
```

## Service Features

The systemd service provides several important features:

- **Automatic Restart**: If the bot crashes, it will automatically restart after 10 seconds
- **Boot Persistence**: The bot will start automatically when the system boots (if enabled)
- **Session Independence**: The bot continues running even if you log out or your SSH connection drops
- **Centralized Logging**: All output is captured in systemd's journal
- **Environment Management**: Environment variables are loaded from the `.env` file

## Troubleshooting

### Service Won't Start

1. **Check service status for errors:**
   ```bash
   sudo systemctl status persona.service
   ```

2. **View detailed logs:**
   ```bash
   sudo journalctl -u persona.service -n 50
   ```

3. **Verify the binary exists and is executable:**
   ```bash
   ls -la /home/caavere/Projects/bot/persona/target/release/persona
   ```

4. **Check .env file permissions:**
   ```bash
   ls -la /home/caavere/Projects/bot/persona/.env
   ```

### Environment Variables Not Loading

If the bot can't read environment variables:

1. **Verify .env file exists:**
   ```bash
   cat /home/caavere/Projects/bot/persona/.env
   ```

2. **Check file permissions:**
   ```bash
   chmod 600 /home/caavere/Projects/bot/persona/.env
   ```

3. **Ensure the .env file format is correct** (no spaces around `=`, one variable per line)

### Permission Errors

If you see permission errors:

1. **Verify the User and Group in the service file match your username:**
   ```bash
   whoami  # Should match the User field in persona.service
   ```

2. **Check ownership of project files:**
   ```bash
   ls -la /home/caavere/Projects/bot/persona/
   ```

### Updating the Bot

When you update the code:

**Using Makefile:**
```bash
# Rebuild and restart in one command
make update
```

**Manual steps:**
1. **Rebuild the release binary:**
   ```bash
   cargo build --release
   ```

2. **Restart the service:**
   ```bash
   sudo systemctl restart persona.service
   ```

3. **Verify it's running with the new code:**
   ```bash
   sudo systemctl status persona.service
   sudo journalctl -u persona.service -f
   ```

## Alternative: Running with nohup

If you prefer not to use systemd, you can use `nohup` as a simpler alternative:

```bash
nohup /home/caavere/Projects/bot/persona/target/release/persona > persona.log 2>&1 &
```

However, systemd is recommended as it provides better management, logging, and automatic restart capabilities.
