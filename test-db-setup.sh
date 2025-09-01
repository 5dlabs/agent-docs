#!/bin/bash

# Local Database Testing Setup Script
# This script helps set up and run database tests locally

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

DB_URL="postgresql://test_user:test_password@localhost:5433/test_db"

echo "🧪 Local Database Test Setup"
echo "============================="

# Function to start database
start_db() {
    echo "🐳 Starting PostgreSQL test database..."
    docker-compose -f docker-compose.test.yml up -d

    echo "⏳ Waiting for database to be ready..."
    for i in {1..30}; do
        if docker-compose -f docker-compose.test.yml exec -T postgres pg_isready -U test_user -d test_db >/dev/null 2>&1; then
            echo "✅ Database is ready!"
            return 0
        fi
        echo "Waiting... ($i/30)"
        sleep 2
    done

    echo "❌ Database failed to start within 60 seconds"
    return 1
}

# Function to stop database
stop_db() {
    echo "🛑 Stopping PostgreSQL test database..."
    docker-compose -f docker-compose.test.yml down
    echo "✅ Database stopped"
}

# Function to setup schema
setup_schema() {
    echo "📋 Setting up database schema..."
    if docker-compose -f docker-compose.test.yml exec -T postgres psql -U test_user -d test_db -f /dev/stdin < scripts/setup_test_db.sql; then
        echo "✅ Schema setup completed"
    else
        echo "❌ Schema setup failed"
        return 1
    fi
}

# Function to run tests
run_tests() {
    echo "🧪 Running database tests..."
    TEST_DATABASE_URL="$DB_URL" cargo test -p db --test crate_operations -- --nocapture
}

# Function to show status
show_status() {
    echo "📊 Database Status:"
    if docker-compose -f docker-compose.test.yml ps | grep -q "Up"; then
        echo "✅ Database container is running"
        if docker-compose -f docker-compose.test.yml exec -T postgres pg_isready -U test_user -d test_db >/dev/null 2>&1; then
            echo "✅ Database is ready for connections"
            echo "🔗 Connection URL: $DB_URL"
        else
            echo "❌ Database is not responding"
        fi
    else
        echo "❌ Database container is not running"
    fi
}

# Main command handling
case "${1:-help}" in
    "start")
        start_db
        setup_schema
        ;;
    "stop")
        stop_db
        ;;
    "status")
        show_status
        ;;
    "test")
        if ! docker-compose -f docker-compose.test.yml ps | grep -q "Up"; then
            echo "❌ Database is not running. Run '$0 start' first."
            exit 1
        fi
        run_tests
        ;;
    "restart")
        stop_db
        sleep 2
        start_db
        setup_schema
        ;;
    "shell")
        echo "🐚 Connecting to database shell..."
        docker-compose -f docker-compose.test.yml exec postgres psql -U test_user -d test_db
        ;;
    "help"|*)
        echo "Usage: $0 <command>"
        echo ""
        echo "Commands:"
        echo "  start   - Start database and setup schema"
        echo "  stop    - Stop database"
        echo "  status  - Show database status"
        echo "  test    - Run database tests"
        echo "  restart - Restart database and setup schema"
        echo "  shell   - Connect to database shell"
        echo "  help    - Show this help message"
        echo ""
        echo "Examples:"
        echo "  $0 start    # Start everything"
        echo "  $0 test     # Run tests"
        echo "  $0 stop     # Clean up"
        ;;
esac
