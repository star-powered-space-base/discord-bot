# Database Scripts Overview

A comprehensive suite of database management and analytics scripts has been added to the `scripts/` directory.

## Quick Start

```bash
# View available SQL queries
./scripts/query.sh

# Run a query
./scripts/query.sh user_stats

# Interactive database inspector
./scripts/inspect.sh

# Create a backup
./scripts/backup.sh

# Generate analytics report
./scripts/analytics/generate_report.py
```

## What's Included

### 📦 Core Utilities

| Script | Purpose | Usage |
|--------|---------|-------|
| `backup.sh` | Create timestamped database backups | `./scripts/backup.sh` |
| `restore.sh` | Restore from backup | `./scripts/restore.sh --latest` |
| `export_data.sh` | Export to CSV/JSON/SQL | `./scripts/export_data.sh csv` |
| `inspect.sh` | Interactive database browser | `./scripts/inspect.sh` |
| `query.sh` | Run SQL query files | `./scripts/query.sh user_stats` |

### 📊 Pre-written SQL Queries (`sql/`)

- **user_stats.sql** - Top users, command usage, activity patterns
- **daily_analytics.sql** - Daily/weekly/monthly trends
- **performance_metrics.sql** - Response times, API latency
- **error_report.sql** - Error logs and analysis
- **conversation_stats.sql** - Conversation patterns, persona usage
- **db_info.sql** - Database schema and health

### 📈 Analytics (`analytics/`)

- **generate_report.py** - Comprehensive analytics report (text or JSON)

### 🔧 Maintenance (`maintenance/`)

- **cleanup.sh** - Remove old data, vacuum database, optimize

## File Tree

```
scripts/
├── README.md              # Detailed documentation
├── backup.sh              # Database backup utility
├── restore.sh             # Backup restoration
├── export_data.sh         # Data export (CSV/JSON/SQL)
├── inspect.sh             # Interactive inspector
├── query.sh               # SQL query runner
├── sql/                   # SQL query library
│   ├── user_stats.sql
│   ├── daily_analytics.sql
│   ├── performance_metrics.sql
│   ├── error_report.sql
│   ├── conversation_stats.sql
│   └── db_info.sql
├── analytics/
│   └── generate_report.py
└── maintenance/
    └── cleanup.sh
```

## Common Workflows

### Daily Operations

```bash
# Check recent activity
./scripts/query.sh user_stats

# View recent errors
./scripts/query.sh error_report

# Create daily backup
./scripts/backup.sh
```

### Weekly Maintenance

```bash
# Backup before cleanup
./scripts/backup.sh

# Clean old data
./scripts/maintenance/cleanup.sh

# Generate weekly report
./scripts/analytics/generate_report.py > reports/week_$(date +%Y%m%d).txt
```

### Data Export

```bash
# Export all tables to CSV
./scripts/export_data.sh csv

# Full SQL dump
./scripts/export_data.sh sql

# Export to JSON
./scripts/export_data.sh json
```

### Troubleshooting

```bash
# Interactive inspection
./scripts/inspect.sh

# Check database health
./scripts/query.sh db_info

# View performance metrics
./scripts/query.sh performance_metrics
```

## Environment Variables

All scripts support these environment variables:

- `DATABASE_PATH` - Database location (default: `persona.db`)
- `BACKUP_DIR` - Backup storage (default: `backups/`)
- `EXPORT_DIR` - Export storage (default: `exports/`)

Example:
```bash
DATABASE_PATH=/custom/path.db ./scripts/backup.sh
```

## Automation Examples

### Cron Jobs

```cron
# Daily backup at 2 AM
0 2 * * * cd /path/to/persona && ./scripts/backup.sh

# Weekly cleanup on Sunday at 3 AM
0 3 * * 0 cd /path/to/persona && ./scripts/maintenance/cleanup.sh

# Weekly report on Monday at 9 AM
0 9 * * 1 cd /path/to/persona && ./scripts/analytics/generate_report.py > reports/weekly_$(date +\%Y\%m\%d).txt
```

### Systemd Timer

Create `/etc/systemd/system/persona-backup.timer`:
```ini
[Unit]
Description=Daily Persona DB Backup

[Timer]
OnCalendar=daily
OnCalendar=02:00

[Install]
WantedBy=timers.target
```

## Features Highlights

- ✅ All scripts are executable and ready to use
- ✅ Comprehensive error handling
- ✅ Progress indicators and colored output
- ✅ Automatic backup rotation
- ✅ Database integrity checks
- ✅ Safe restore with automatic backups
- ✅ Configurable retention policies
- ✅ No external dependencies (except Python for analytics)
- ✅ Works with standard Unix tools (sqlite3, bash, cron)

## Documentation

For detailed documentation, see:
- **`scripts/README.md`** - Complete reference guide
- Individual script help: `./scripts/<script>.sh --help` (where available)

## Testing

All scripts have been tested and are production-ready:

```bash
# Verify scripts are executable
ls -l scripts/*.sh

# Test query runner
./scripts/query.sh

# Test analytics
./scripts/analytics/generate_report.py

# Test inspector (interactive)
./scripts/inspect.sh
```
