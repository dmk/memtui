# Testing Guide

## Unit Tests

Unit tests are tests that don't require external dependencies (like a running Redis instance). They test pure functions, configuration building, and error handling.

### Running Unit Tests

```bash
cargo test --lib
```

This will run all unit tests including:
- Connection URL building with various configurations
- Redis type mapping
- JSON detection
- INFO response parsing
- Error conversion
- Capabilities checking

## Integration Tests

Integration tests require a running Redis instance and test the actual backend operations against a real Redis server.

### Prerequisites

1. Install and start Redis:
   ```bash
   # On macOS
   brew install redis
   brew services start redis

   # On Ubuntu/Debian
   sudo apt-get install redis-server
   sudo systemctl start redis

   # Using Docker
   docker run -d -p 6379:6379 redis:latest
   ```

2. (Optional) Set environment variables:
   ```bash
   export REDIS_HOST=localhost
   export REDIS_PORT=6379
   export REDIS_PASSWORD=your_password  # if Redis requires authentication
   ```

### Running Integration Tests

```bash
cargo test --lib --features integration-tests
```

This will run all integration tests including:
- Connection and disconnection
- Ping operations
- Server info retrieval
- Set and get operations
- Key scanning and counting
- TTL handling
- JSON detection
- Batch operations (get_many)
- Delete operations
- Raw command execution
- Read-only mode enforcement

### Integration Test Details

- Tests use database 15 to avoid interfering with other data
- Each test cleans up by running `FLUSHDB` on the test database
- Tests verify actual Redis operations work as expected
- Tests check error handling for invalid operations

## Running All Tests

```bash
# Unit tests only (no Redis required)
make test

# All tests including integration tests (Redis required)
cargo test --all-features
```

## Continuous Integration

For CI environments, you can use the Redis Docker container:

```yaml
# Example GitHub Actions
services:
  redis:
    image: redis:latest
    ports:
      - 6379:6379
    options: >-
      --health-cmd "redis-cli ping"
      --health-interval 10s
      --health-timeout 5s
      --health-retries 5

# Then run tests with:
# cargo test --all-features
```

## Code Coverage

To generate code coverage reports:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --lib --out Html --features integration-tests
```

Note: Integration tests require a running Redis instance even for coverage reports.

