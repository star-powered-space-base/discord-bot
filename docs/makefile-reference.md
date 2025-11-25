# Makefile Quick Reference

This document provides a quick reference for all available Makefile targets.

## Getting Started

To see all available commands at any time:
```bash
make help
```

## Development Commands

| Command | Description |
|---------|-------------|
| `make build` | Build the project in debug mode |
| `make build-release` | Build the project in release mode (optimized) |
| `make run` | Run the bot in development mode |
| `make run-release` | Run the bot in release mode |
| `make test` | Run tests |
| `make clean` | Clean build artifacts |
| `make check` | Check code without building |
| `make fmt` | Format code with rustfmt |
| `make lint` | Run clippy linter |

## Deployment Commands

| Command | Description |
|---------|-------------|
| `make install-service` | Install systemd service (requires sudo) |
| `make uninstall-service` | Uninstall systemd service (requires sudo) |
| `make start` | Start the systemd service (requires sudo) |
| `make stop` | Stop the systemd service (requires sudo) |
| `make restart` | Restart the systemd service (requires sudo) |
| `make reload-service` | Reload systemd service after config changes (requires sudo) |
| `make status` | Show systemd service status |

## Logging Commands

| Command | Description |
|---------|-------------|
| `make logs` | View recent service logs (last 100 lines) |
| `make logs-follow` | Follow service logs in real-time |
| `make logs-boot` | View logs since last boot |
| `make logs-export` | Export logs to file |

## Environment Commands

| Command | Description |
|---------|-------------|
| `make env-check` | Check if required environment variables are set |
| `make env-setup` | Copy .env.example to .env |

## Database Commands

| Command | Description |
|---------|-------------|
| `make db-status` | Check database status |
| `make db-backup` | Backup database |

## Complete Workflows

| Command | Description |
|---------|-------------|
| `make setup` | Complete setup: check env, build, and install service |
| `make update` | Update: rebuild and restart service |
| `make reinstall` | Full reinstall: stop, rebuild, and restart |

## Common Workflows

### Initial Setup
```bash
# 1. Create and configure environment file
make env-setup
# Edit .env with your credentials

# 2. Verify environment configuration
make env-check

# 3. Complete setup (builds and installs service)
make setup

# 4. Start the bot
make start

# 5. Watch logs
make logs-follow
```

### Development Workflow
```bash
# Format and lint code
make fmt
make lint

# Build and test
make check
make test

# Run locally for testing
make run
```

### Updating After Code Changes
```bash
# Quick update (rebuild and restart)
make update

# Or manually
make build-release
make restart
```

### Troubleshooting
```bash
# Check service status
make status

# View recent logs
make logs

# Follow logs in real-time
make logs-follow

# Export logs for analysis
make logs-export

# Check database
make db-status

# Backup database before making changes
make db-backup
```

### Uninstalling
```bash
# Stop and remove the service
make stop
make uninstall-service

# Clean build artifacts
make clean
```

## Scripts Commands

The project includes organized scripts in the `scripts/` directory with Makefile delegation:

| Command | Description |
|---------|-------------|
| `make scripts/commands/check` | Check registered Discord commands |
| `make scripts/commands/cleanup` | Remove duplicate command registrations |
| `make scripts/service/reload` | Reload systemd service |
| `make scripts/service/status` | Check service status |
| `make scripts/tunnel/setup` | Configure ngrok tunnel |
| `make scripts/tunnel/start-http` | Start bot with HTTP tunnel |
| `make scripts/tunnel/start-gateway` | Start bot with gateway tunnel |
| `make scripts/test/env` | Validate environment configuration |
| `make scripts/test/openai` | Test OpenAI API connectivity |

### Convenience Aliases

| Alias | Equivalent |
|-------|------------|
| `make check-commands` | `make scripts/commands/check` |
| `make cleanup-commands` | `make scripts/commands/cleanup` |

See `scripts/README.md` for detailed script documentation.

## Tips

- Run `make` or `make help` at any time to see all available commands
- Most deployment commands require `sudo` for systemd operations
- The `setup` command automates the entire installation process
- Use `logs-follow` to monitor the bot in real-time
- Always run `env-check` to verify your configuration before deployment
- Use `db-backup` before making any database schema changes
