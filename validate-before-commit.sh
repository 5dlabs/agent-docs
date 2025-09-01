#!/bin/bash

# Validate Before Commit Script
# Ensures everything works locally before going to CI

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🔍 Pre-Commit Validation"
echo "========================"

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# 1. Check if required tools are available
echo "📋 Checking required tools..."
if ! command_exists docker; then
    echo "❌ Docker is not installed"
    exit 1
fi

if ! command_exists cargo; then
    echo "❌ Cargo is not installed"
    exit 1
fi

echo "✅ All required tools are available"

# 2. Run linting
echo ""
echo "🔧 Running Clippy..."
if ! cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic >/dev/null 2>&1; then
    echo "❌ Clippy failed"
    cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic
    exit 1
fi
echo "✅ Clippy passed"

# 3. Run non-database tests
echo ""
echo "🧪 Running non-database tests..."
if ! cargo test --workspace --exclude db -- --nocapture >/dev/null 2>&1; then
    echo "❌ Non-database tests failed"
    cargo test --workspace --exclude db -- --nocapture
    exit 1
fi
echo "✅ Non-database tests passed"

# 4. Check database test behavior
echo ""
echo "💾 Checking database test behavior..."

echo "Local environment (should skip gracefully):"
LOCAL_RESULT=$(cargo test -p db --test crate_operations -- --nocapture 2>/dev/null | grep -E "(test result)" | tail -1)
if echo "$LOCAL_RESULT" | grep -q "0 failed"; then
    echo "✅ Local database tests skip gracefully"
else
    echo "❌ Local database tests are failing"
    exit 1
fi

echo "CI simulation (should run or skip appropriately):"
CI_RESULT=$(unset TEST_DATABASE_URL && cargo test -p db --test crate_operations -- --nocapture 2>/dev/null | grep -E "(test result)" | tail -1)
if echo "$CI_RESULT" | grep -q "0 failed"; then
    echo "✅ CI simulation works correctly"
else
    echo "❌ CI simulation failed"
    exit 1
fi

# 5. Optional: Run with local database if available
if docker-compose -f docker-compose.test.yml ps | grep -q "Up" >/dev/null 2>&1; then
    echo ""
    echo "🐳 Local database is running - running full database tests..."
    if ./test-db-setup.sh test >/dev/null 2>&1; then
        echo "✅ Full database tests with local DB passed"
    else
        echo "❌ Full database tests with local DB failed"
        exit 1
    fi
else
    echo ""
    echo "💡 Tip: Run './test-db-setup.sh start' to test with a real database locally"
fi

echo ""
echo "🎉 All validations passed! Ready for commit."
echo ""
echo "Summary:"
echo "- ✅ Clippy: Clean"
echo "- ✅ Unit Tests: Passing"
echo "- ✅ Database Tests: Skip gracefully locally"
echo "- ✅ CI Simulation: Works correctly"
