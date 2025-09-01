#!/bin/bash

# Comprehensive acceptance testing suite for Task 13 production deployment
# Tests all functional and performance requirements

set -euo pipefail

# Configuration
BASE_URL="${BASE_URL:-http://localhost:3001}"
EXPECTED_TOOLS=13  # 9 query tools + 4 management tools
MAX_RESPONSE_TIME=2.0  # 2 seconds requirement
CONCURRENT_CONNECTIONS=100
MIN_SUCCESS_RATE=95.0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
TEST_RESULTS=()

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    TEST_RESULTS+=("✅ $1")
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    TEST_RESULTS+=("❌ $1")
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check if server is running
check_server_health() {
    log_info "Checking server health..."
    
    if curl -sf "$BASE_URL/health" > /dev/null; then
        log_success "Server health check passed"
        return 0
    else
        log_error "Server health check failed - server not responding"
        return 1
    fi
}

# Test HTTP transport - POST method (JSON-RPC)
test_http_transport_post() {
    log_info "Testing HTTP transport POST method..."
    
    local response
    local http_code
    
    response=$(curl -s -w "HTTPSTATUS:%{http_code}" \
        -X POST "$BASE_URL/mcp" \
        -H "Content-Type: application/json" \
        -H "MCP-Protocol-Version: 2025-06-18" \
        -d '{"method": "tools/list", "params": {}}' 2>/dev/null)
    
    http_code=$(echo "$response" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
    response_body=$(echo "$response" | sed -e 's/HTTPSTATUS:.*//g')
    
    if [[ "$http_code" == "200" ]]; then
        log_success "HTTP POST transport working (status: $http_code)"
        
        # Validate JSON response
        if echo "$response_body" | jq -e '.result.tools' >/dev/null 2>&1; then
            log_success "JSON-RPC response format valid"
        else
            log_error "Invalid JSON-RPC response format"
        fi
    else
        log_error "HTTP POST transport failed (status: $http_code)"
        echo "Response: $response_body"
    fi
}

# Test HTTP transport - GET method (should return 405)
test_http_transport_get() {
    log_info "Testing HTTP transport GET method (should return 405)..."
    
    local response
    local http_code
    
    response=$(curl -s -w "HTTPSTATUS:%{http_code}" \
        -X GET "$BASE_URL/mcp" \
        -H "MCP-Protocol-Version: 2025-06-18" 2>/dev/null)
    
    http_code=$(echo "$response" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
    
    if [[ "$http_code" == "405" ]]; then
        log_success "GET method correctly returns 405 Method Not Allowed"
    else
        log_error "GET method should return 405, got: $http_code"
    fi
}

# Test all available tools
test_tool_availability() {
    log_info "Testing tool availability..."
    
    local response
    response=$(curl -s -X POST "$BASE_URL/mcp" \
        -H "Content-Type: application/json" \
        -H "MCP-Protocol-Version: 2025-06-18" \
        -d '{"method": "tools/list", "params": {}}')
    
    local tool_count
    tool_count=$(echo "$response" | jq -r '.result.tools | length' 2>/dev/null || echo "0")
    
    if [[ "$tool_count" -ge "$EXPECTED_TOOLS" ]]; then
        log_success "All $tool_count tools available (expected: $EXPECTED_TOOLS)"
    else
        log_error "Only $tool_count tools found, expected at least $EXPECTED_TOOLS"
        echo "Available tools:"
        echo "$response" | jq -r '.result.tools[].name' 2>/dev/null | sed 's/^/  - /'
    fi
    
    # Test specific query tools
    local query_tools
    query_tools=$(echo "$response" | jq -r '.result.tools[] | select(.name | endswith("_query")) | .name' 2>/dev/null)
    local query_count
    query_count=$(echo "$query_tools" | wc -l)
    
    if [[ "$query_count" -ge 9 ]]; then
        log_success "All $query_count query tools available"
    else
        log_error "Only $query_count query tools found, expected at least 9"
    fi
    
    echo "Query tools found:"
    echo "$query_tools" | sed 's/^/  - /'
}

# Test individual query tools
test_query_tools() {
    log_info "Testing individual query tools..."
    
    # Get list of query tools
    local response
    response=$(curl -s -X POST "$BASE_URL/mcp" \
        -H "Content-Type: application/json" \
        -H "MCP-Protocol-Version: 2025-06-18" \
        -d '{"method": "tools/list", "params": {}}')
    
    local query_tools
    query_tools=$(echo "$response" | jq -r '.result.tools[] | select(.name | endswith("_query")) | .name' 2>/dev/null)
    
    local tool_failures=0
    
    while read -r tool; do
        if [[ -n "$tool" ]]; then
            log_info "Testing $tool..."
            
            local test_response
            local start_time
            local end_time
            local duration
            
            start_time=$(date +%s.%N)
            test_response=$(curl -s -X POST "$BASE_URL/mcp" \
                -H "Content-Type: application/json" \
                -H "MCP-Protocol-Version: 2025-06-18" \
                -d "{\"method\": \"tools/call\", \"params\": {\"name\": \"$tool\", \"arguments\": {\"query\": \"test\"}}}" 2>/dev/null)
            end_time=$(date +%s.%N)
            
            duration=$(echo "$end_time - $start_time" | bc -l)
            
            if echo "$test_response" | jq -e '.result' >/dev/null 2>&1; then
                if (( $(echo "$duration < $MAX_RESPONSE_TIME" | bc -l) )); then
                    log_success "$tool working (${duration}s)"
                else
                    log_error "$tool working but slow (${duration}s > ${MAX_RESPONSE_TIME}s)"
                    tool_failures=$((tool_failures + 1))
                fi
            else
                log_error "$tool failed"
                echo "Response: $test_response"
                tool_failures=$((tool_failures + 1))
            fi
        fi
    done <<< "$query_tools"
    
    if [[ $tool_failures -eq 0 ]]; then
        log_success "All query tools working within performance requirements"
    else
        log_error "$tool_failures query tools failed performance or functionality tests"
    fi
}

# Test Rust crate management tools
test_rust_management_tools() {
    log_info "Testing Rust crate management tools..."
    
    # Test list_rust_crates
    local response
    response=$(curl -s -X POST "$BASE_URL/mcp" \
        -H "Content-Type: application/json" \
        -H "MCP-Protocol-Version: 2025-06-18" \
        -d '{"method": "tools/call", "params": {"name": "list_rust_crates", "arguments": {}}}')
    
    if echo "$response" | jq -e '.result' >/dev/null 2>&1; then
        log_success "list_rust_crates working"
    else
        log_error "list_rust_crates failed"
        echo "Response: $response"
    fi
    
    # Test check_rust_status
    response=$(curl -s -X POST "$BASE_URL/mcp" \
        -H "Content-Type: application/json" \
        -H "MCP-Protocol-Version: 2025-06-18" \
        -d '{"method": "tools/call", "params": {"name": "check_rust_status", "arguments": {}}}')
    
    if echo "$response" | jq -e '.result' >/dev/null 2>&1; then
        log_success "check_rust_status working"
    else
        log_error "check_rust_status failed"
        echo "Response: $response"
    fi
}

# Performance testing - response time requirement
test_response_time_performance() {
    log_info "Testing response time performance (< ${MAX_RESPONSE_TIME}s)..."
    
    local total_time=0
    local test_count=10
    local slow_requests=0
    
    for i in $(seq 1 $test_count); do
        local start_time
        local end_time
        local duration
        
        start_time=$(date +%s.%N)
        curl -s -X POST "$BASE_URL/mcp" \
            -H "Content-Type: application/json" \
            -H "MCP-Protocol-Version: 2025-06-18" \
            -d '{"method": "tools/call", "params": {"name": "rust_query", "arguments": {"query": "serde"}}}' > /dev/null
        end_time=$(date +%s.%N)
        
        duration=$(echo "$end_time - $start_time" | bc -l)
        total_time=$(echo "$total_time + $duration" | bc -l)
        
        if (( $(echo "$duration > $MAX_RESPONSE_TIME" | bc -l) )); then
            slow_requests=$((slow_requests + 1))
            log_warning "Request $i took ${duration}s (> ${MAX_RESPONSE_TIME}s)"
        fi
    done
    
    local avg_time
    avg_time=$(echo "scale=3; $total_time / $test_count" | bc -l)
    
    if [[ $slow_requests -eq 0 ]]; then
        log_success "All requests under ${MAX_RESPONSE_TIME}s (avg: ${avg_time}s)"
    else
        log_error "$slow_requests/$test_count requests exceeded ${MAX_RESPONSE_TIME}s threshold (avg: ${avg_time}s)"
    fi
}

# Load testing - concurrent connections
test_concurrent_connections() {
    log_info "Testing concurrent connections (${CONCURRENT_CONNECTIONS} concurrent)..."
    
    # Use GNU parallel if available, otherwise use a simple background job approach
    if command -v parallel >/dev/null 2>&1; then
        log_info "Using GNU parallel for load testing..."
        
        local success_count
        success_count=$(seq 1 $CONCURRENT_CONNECTIONS | parallel -j $CONCURRENT_CONNECTIONS \
            "curl -s -X POST '$BASE_URL/mcp' \
             -H 'Content-Type: application/json' \
             -H 'MCP-Protocol-Version: 2025-06-18' \
             -d '{\"method\": \"tools/list\", \"params\": {}}' \
             -o /dev/null -w '%{http_code}\n'" | \
            grep '^200$' | wc -l)
        
        local success_rate
        success_rate=$(echo "scale=1; $success_count * 100 / $CONCURRENT_CONNECTIONS" | bc -l)
        
        if (( $(echo "$success_rate >= $MIN_SUCCESS_RATE" | bc -l) )); then
            log_success "Concurrent connections test passed (${success_rate}% success rate)"
        else
            log_error "Concurrent connections test failed (${success_rate}% success rate, need ${MIN_SUCCESS_RATE}%)"
        fi
    else
        log_warning "GNU parallel not available, using simplified concurrent test..."
        
        local pids=()
        local success_count=0
        
        # Start concurrent requests
        for i in $(seq 1 20); do  # Test with 20 concurrent requests instead
            {
                if curl -s -X POST "$BASE_URL/mcp" \
                    -H "Content-Type: application/json" \
                    -H "MCP-Protocol-Version: 2025-06-18" \
                    -d '{"method": "tools/list", "params": {}}' \
                    -o /dev/null; then
                    echo "success" > "/tmp/load_test_$i"
                else
                    echo "failure" > "/tmp/load_test_$i"
                fi
            } &
            pids+=($!)
        done
        
        # Wait for all requests to complete
        for pid in "${pids[@]}"; do
            wait "$pid"
        done
        
        # Count successes
        for i in $(seq 1 20); do
            if [[ -f "/tmp/load_test_$i" && $(cat "/tmp/load_test_$i") == "success" ]]; then
                success_count=$((success_count + 1))
            fi
            rm -f "/tmp/load_test_$i"
        done
        
        local success_rate
        success_rate=$(echo "scale=1; $success_count * 100 / 20" | bc -l)
        
        if (( $(echo "$success_rate >= $MIN_SUCCESS_RATE" | bc -l) )); then
            log_success "Simplified concurrent test passed (${success_rate}% success rate with 20 concurrent)"
        else
            log_error "Simplified concurrent test failed (${success_rate}% success rate)"
        fi
    fi
}

# Test protocol version validation
test_protocol_version() {
    log_info "Testing protocol version validation..."
    
    # Test with correct version
    local response
    response=$(curl -s -w "HTTPSTATUS:%{http_code}" \
        -X POST "$BASE_URL/mcp" \
        -H "Content-Type: application/json" \
        -H "MCP-Protocol-Version: 2025-06-18" \
        -d '{"method": "tools/list", "params": {}}')
    
    local http_code
    http_code=$(echo "$response" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
    
    if [[ "$http_code" == "200" ]]; then
        log_success "Correct protocol version accepted (2025-06-18)"
    else
        log_error "Correct protocol version rejected (status: $http_code)"
    fi
    
    # Test with incorrect version
    response=$(curl -s -w "HTTPSTATUS:%{http_code}" \
        -X POST "$BASE_URL/mcp" \
        -H "Content-Type: application/json" \
        -H "MCP-Protocol-Version: 2024-01-01" \
        -d '{"method": "tools/list", "params": {}}')
    
    http_code=$(echo "$response" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
    
    if [[ "$http_code" == "400" ]]; then
        log_success "Incorrect protocol version rejected (status: $http_code)"
    else
        log_error "Incorrect protocol version should be rejected with 400, got: $http_code"
    fi
}

# Test error handling
test_error_handling() {
    log_info "Testing error handling..."
    
    # Test invalid JSON
    local response
    response=$(curl -s -w "HTTPSTATUS:%{http_code}" \
        -X POST "$BASE_URL/mcp" \
        -H "Content-Type: application/json" \
        -H "MCP-Protocol-Version: 2025-06-18" \
        -d 'invalid json')
    
    local http_code
    http_code=$(echo "$response" | tr -d '\n' | sed -e 's/.*HTTPSTATUS://')
    
    if [[ "$http_code" == "400" ]]; then
        log_success "Invalid JSON properly rejected (status: $http_code)"
    else
        log_error "Invalid JSON should return 400, got: $http_code"
    fi
    
    # Test invalid tool name
    response=$(curl -s -X POST "$BASE_URL/mcp" \
        -H "Content-Type: application/json" \
        -H "MCP-Protocol-Version: 2025-06-18" \
        -d '{"method": "tools/call", "params": {"name": "nonexistent_tool", "arguments": {}}}')
    
    if echo "$response" | jq -e '.error' >/dev/null 2>&1; then
        log_success "Invalid tool name properly returns error"
    else
        log_error "Invalid tool name should return error"
        echo "Response: $response"
    fi
}

# Generate summary report
generate_summary() {
    echo
    echo "=========================================="
    echo "         ACCEPTANCE TEST SUMMARY"
    echo "=========================================="
    echo
    
    for result in "${TEST_RESULTS[@]}"; do
        echo "$result"
    done
    
    echo
    echo "Total Tests: $((TESTS_PASSED + TESTS_FAILED))"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    
    local pass_rate
    pass_rate=$(echo "scale=1; $TESTS_PASSED * 100 / ($TESTS_PASSED + $TESTS_FAILED)" | bc -l)
    echo "Pass Rate: ${pass_rate}%"
    echo
    
    if [[ $TESTS_FAILED -eq 0 ]]; then
        echo -e "${GREEN}🎉 ALL ACCEPTANCE CRITERIA MET!${NC}"
        echo -e "${GREEN}✅ Production deployment validation successful${NC}"
        return 0
    else
        echo -e "${RED}❌ ACCEPTANCE CRITERIA NOT MET${NC}"
        echo -e "${RED}Production deployment validation failed${NC}"
        return 1
    fi
}

# Main test execution
main() {
    log_info "Starting comprehensive acceptance testing for Task 13..."
    log_info "Base URL: $BASE_URL"
    echo
    
    # Core functionality tests
    check_server_health
    test_http_transport_post
    test_http_transport_get
    test_protocol_version
    
    # Tool availability and functionality
    test_tool_availability
    test_query_tools
    test_rust_management_tools
    
    # Performance tests
    test_response_time_performance
    test_concurrent_connections
    
    # Error handling
    test_error_handling
    
    # Generate final report
    generate_summary
}

# Check dependencies
if ! command -v jq >/dev/null 2>&1; then
    log_error "jq is required but not installed. Please install jq."
    exit 1
fi

if ! command -v bc >/dev/null 2>&1; then
    log_error "bc is required but not installed. Please install bc."
    exit 1
fi

# Run main function
main "$@"