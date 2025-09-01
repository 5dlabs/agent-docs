# Local Database Testing

This guide explains how to run database tests locally using Docker and PostgreSQL.

## Prerequisites

- Docker and Docker Compose installed
- The `test-db-setup.sh` script (already created)

## Quick Start

### 1. Start the Database
```bash
./test-db-setup.sh start
```

This will:
- Start a PostgreSQL container on port 5433
- Set up the database schema automatically
- Wait for the database to be ready

### 2. Run Database Tests
```bash
./test-db-setup.sh test
```

This will run all the database tests with the local PostgreSQL instance.

### 3. Stop the Database
```bash
./test-db-setup.sh stop
```

## Available Commands

| Command | Description |
|---------|-------------|
| `./test-db-setup.sh start` | Start database and setup schema |
| `./test-db-setup.sh stop` | Stop database |
| `./test-db-setup.sh status` | Show database status |
| `./test-db-setup.sh test` | Run database tests |
| `./test-db-setup.sh restart` | Restart database and setup schema |
| `./test-db-setup.sh shell` | Connect to database shell |
| `./test-db-setup.sh help` | Show help message |

## Database Configuration

- **Host**: localhost
- **Port**: 5433 (to avoid conflicts with local PostgreSQL)
- **Database**: test_db
- **User**: test_user
- **Password**: test_password
- **Connection URL**: `postgresql://test_user:test_password@localhost:5433/test_db`

## Troubleshooting

### Database Won't Start
```bash
# Check Docker status
docker ps

# Check Docker Compose logs
docker-compose -f docker-compose.test.yml logs

# Restart the database
./test-db-setup.sh restart
```

### Tests Still Failing
```bash
# Check database status
./test-db-setup.sh status

# Connect to database manually
./test-db-setup.sh shell

# Inside the shell, check tables:
# \dt
# SELECT * FROM documents LIMIT 5;
```

### Port Conflict
If port 5433 is already in use:
```yaml
# Edit docker-compose.test.yml and change:
ports:
  - "5434:5432"  # Use a different port
```

Then update the connection URL in `test-db-setup.sh`:
```bash
DB_URL="postgresql://test_user:test_password@localhost:5434/test_db"
```

## Manual Testing

You can also run tests manually with the local database:

```bash
# Set the environment variable
export TEST_DATABASE_URL="postgresql://test_user:test_password@localhost:5433/test_db"

# Run specific tests
cargo test -p db --test crate_operations test_crate_document_metadata_queries -- --nocapture

# Run all database tests
cargo test -p db --test crate_operations -- --nocapture
```

## Data Persistence

The database data is persisted in a Docker volume called `test_db_data`. To completely reset:

```bash
# Stop and remove everything
./test-db-setup.sh stop

# Remove the volume
docker volume rm agent-docs-test-db_test_db_data

# Start fresh
./test-db-setup.sh start
```

## CI vs Local Differences

The local setup replicates the CI environment as closely as possible:

- ✅ Same PostgreSQL version (15)
- ✅ Same schema setup script
- ✅ Same Rust test code
- ✅ Same environment variable pattern

This should help identify if issues are:
- **Database-related**: Fixed by local testing
- **CI-specific**: Network, permissions, or configuration issues
- **Code-related**: Logic errors that affect both environments
