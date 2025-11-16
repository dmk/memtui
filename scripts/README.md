# Scripts

This directory contains utility scripts for the memtui project.

## seed_datastores.py

Python script to seed Redis and Memcached datastores with realistic sample data.

### Installation

First, install the required dependencies:

```bash
pip install -r scripts/requirements.txt
```

### Usage

Basic usage:

```bash
./scripts/seed_datastores.py
```

With custom scale:

```bash
# Generate 2x the default amount of data
./scripts/seed_datastores.py --scale 2.0

# Generate half the default amount
./scripts/seed_datastores.py --scale 0.5

# Flush existing data and seed fresh (WARNING: destructive!)
./scripts/seed_datastores.py --flush

# Large dataset for stress testing (10x)
./scripts/seed_datastores.py --scale 10.0 --seed 42 --verbose
```

### CLI Options

- `--redis-host HOST`: Redis host address (default: 127.0.0.1)
- `--redis-port PORT`: Redis port (default: 6379)
- `--memcached-host HOST`: Memcached host address (default: 127.0.0.1)
- `--memcached-port PORT`: Memcached port (default: 11211)
- `--scale N`: Scale multiplier for all data (default: 1.0)
  - Base amounts: 200 Redis users, 120 Memcached users, 40 jobs
  - Use 0.5 for half, 2.0 for double, 10.0 for 10x, etc.
- `--seed N`: Random seed for reproducible data generation
- `--flush`: Flush all existing data before seeding (WARNING: destructive)
- `-v, --verbose`: Enable verbose output

### Features

- **Simple scaling**: Single `--scale` parameter controls all data volumes proportionally
- **Colorful output**: Beautiful colored terminal output with progress bars and status indicators
- **Progress bars**: Visual feedback for long-running operations using tqdm
- **Realistic data**: Uses Faker library to generate realistic names, emails, dates, etc.
- **Reproducible**: Use `--seed` for consistent data generation across runs
- **Fast and focused**: Only seeds data, doesn't manage services
