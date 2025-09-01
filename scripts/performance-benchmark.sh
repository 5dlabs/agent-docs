#!/bin/bash

# Performance benchmarking script for Doc Server
# Tests response times, throughput, and resource utilization

set -euo pipefail

# Configuration
BASE_URL="${BASE_URL:-http://localhost:3001}"
BENCHMARK_DURATION=${BENCHMARK_DURATION:-60}  # seconds
WARMUP_DURATION=${WARMUP_DURATION:-10}        # seconds
MAX_RESPONSE_TIME=2.0                         # 2 second requirement
OUTPUT_DIR="./benchmark-results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Test response time with different query complexities
benchmark_response_times() {
    log_info "Benchmarking response times for different query types..."
    
    local results_file="$OUTPUT_DIR/response_times_$TIMESTAMP.json"
    echo "[]" > "$results_file"
    
    # Test scenarios with different complexity levels
    local scenarios=(
        "simple:rust_query:serde"
        "medium:birdeye_query:defi pricing api"
        "complex:solana_query:consensus networking validators"
    )
    
    for scenario in "${scenarios[@]}"; do
        IFS=':' read -ra PARTS <<< "$scenario"
        local complexity="${PARTS[0]}"
        local tool="${PARTS[1]}"
        local query="${PARTS[2]}"
        
        log_info "Testing $complexity query: $tool with '$query'"
        
        local total_time=0
        local count=10
        local successful=0
        local failed=0
        local times=()
        
        for i in $(seq 1 $count); do
            local start_time
            local end_time
            local duration
            local http_code
            
            start_time=$(date +%s.%N)
            http_code=$(curl -s -w "%{http_code}" \
                -X POST "$BASE_URL/mcp" \
                -H "Content-Type: application/json" \
                -H "MCP-Protocol-Version: 2025-06-18" \
                -d "{\"method\": \"tools/call\", \"params\": {\"name\": \"$tool\", \"arguments\": {\"query\": \"$query\"}}}" \
                -o /dev/null)
            end_time=$(date +%s.%N)
            
            duration=$(echo "$end_time - $start_time" | bc -l)
            
            if [[ "$http_code" == "200" ]]; then
                successful=$((successful + 1))
                times+=("$duration")
                total_time=$(echo "$total_time + $duration" | bc -l)
            else
                failed=$((failed + 1))
                log_warning "Request failed with HTTP $http_code"
            fi
        done
        
        if [[ $successful -gt 0 ]]; then
            local avg_time
            local min_time
            local max_time
            
            avg_time=$(echo "scale=3; $total_time / $successful" | bc -l)
            min_time=$(printf '%s\n' "${times[@]}" | sort -n | head -1)
            max_time=$(printf '%s\n' "${times[@]}" | sort -n | tail -1)
            
            # Calculate 95th percentile
            local p95_index
            p95_index=$(echo "scale=0; $successful * 0.95" | bc -l)
            local p95_time
            p95_time=$(printf '%s\n' "${times[@]}" | sort -n | sed -n "${p95_index}p")
            
            # Create JSON result
            local result
            result=$(jq -n \
                --arg complexity "$complexity" \
                --arg tool "$tool" \
                --arg query "$query" \
                --argjson avg "$avg_time" \
                --argjson min "$min_time" \
                --argjson max "$max_time" \
                --argjson p95 "$p95_time" \
                --arg successful "$successful" \
                --arg failed "$failed" \
                --argjson meets_requirement "$(echo "$avg_time < $MAX_RESPONSE_TIME" | bc -l)" \
                '{
                    complexity: $complexity,
                    tool: $tool,
                    query: $query,
                    avg_response_time: $avg,
                    min_response_time: $min,
                    max_response_time: $max,
                    p95_response_time: $p95,
                    successful_requests: $successful,
                    failed_requests: $failed,
                    meets_requirement: ($meets_requirement == 1)
                }')
            
            # Append to results file
            jq ". += [$result]" "$results_file" > "$results_file.tmp" && mv "$results_file.tmp" "$results_file"
            
            if (( $(echo "$avg_time < $MAX_RESPONSE_TIME" | bc -l) )); then
                log_success "$complexity query: avg=${avg_time}s, p95=${p95_time}s (meets requirement)"
            else
                log_error "$complexity query: avg=${avg_time}s, p95=${p95_time}s (exceeds ${MAX_RESPONSE_TIME}s requirement)"
            fi
        else
            log_error "$complexity query: All requests failed"
        fi
    done
    
    log_info "Response time results saved to $results_file"
}

# Throughput benchmarking
benchmark_throughput() {
    log_info "Benchmarking throughput with concurrent connections..."
    
    if ! command -v hey >/dev/null 2>&1; then
        log_warning "hey tool not available, installing..."
        
        if [[ "$OSTYPE" == "linux-gnu"* ]]; then
            curl -sf https://hey-release.s3.us-east-2.amazonaws.com/hey_linux_amd64 -o /tmp/hey
            chmod +x /tmp/hey
            sudo mv /tmp/hey /usr/local/bin/hey 2>/dev/null || mv /tmp/hey ~/hey
            export PATH="$HOME:$PATH"
        else
            log_error "Please install hey tool manually"
            return 1
        fi
    fi
    
    local output_file="$OUTPUT_DIR/throughput_$TIMESTAMP.txt"
    
    # Test with increasing concurrent connections
    local concurrency_levels=(1 5 10 25 50 100)
    
    for concurrency in "${concurrency_levels[@]}"; do
        log_info "Testing throughput with $concurrency concurrent connections..."
        
        local requests=$((concurrency * 20))  # 20 requests per connection
        
        hey -n "$requests" -c "$concurrency" -t 30 \
            -m POST \
            -H "Content-Type: application/json" \
            -H "MCP-Protocol-Version: 2025-06-18" \
            -d '{"method": "tools/list", "params": {}}' \
            "$BASE_URL/mcp" >> "$output_file" 2>&1
        
        echo "--- Concurrency: $concurrency ---" >> "$output_file"
        echo "" >> "$output_file"
    done
    
    log_info "Throughput results saved to $output_file"
    
    # Parse and report key metrics
    local success_rate
    success_rate=$(grep "Success rate" "$output_file" | tail -1 | awk '{print $3}' | tr -d '%')
    
    local avg_response_time
    avg_response_time=$(grep "Average:" "$output_file" | tail -1 | awk '{print $2}' | sed 's/s//')
    
    if (( $(echo "$success_rate >= 95" | bc -l) )) && (( $(echo "$avg_response_time < $MAX_RESPONSE_TIME" | bc -l) )); then
        log_success "Throughput test passed: ${success_rate}% success rate, avg ${avg_response_time}s"
    else
        log_error "Throughput test failed: ${success_rate}% success rate, avg ${avg_response_time}s"
    fi
}

# Memory and CPU monitoring during load
monitor_resources() {
    log_info "Monitoring resource usage during load test..."
    
    local monitor_file="$OUTPUT_DIR/resources_$TIMESTAMP.log"
    local pid_file="/tmp/doc_server.pid"
    
    # Try to find the doc server process
    local server_pid
    server_pid=$(pgrep -f "http_server" | head -1 || echo "")
    
    if [[ -n "$server_pid" ]]; then
        log_info "Found doc server process: $server_pid"
        
        # Monitor resources for 30 seconds during load
        {
            echo "timestamp,cpu_percent,memory_mb,connections"
            for i in $(seq 1 30); do
                local cpu
                local memory
                local connections
                
                cpu=$(ps -p "$server_pid" -o pcpu= 2>/dev/null | tr -d ' ' || echo "0")
                memory=$(ps -p "$server_pid" -o rss= 2>/dev/null | awk '{print $1/1024}' || echo "0")
                connections=$(ss -tuln 2>/dev/null | grep ":3001" | wc -l || echo "0")
                
                echo "$(date +%s),$cpu,$memory,$connections"
                sleep 1
            done
        } > "$monitor_file" &
        
        local monitor_pid=$!
        
        # Run a load test while monitoring
        hey -n 1000 -c 50 -t 30 \
            -m POST \
            -H "Content-Type: application/json" \
            -H "MCP-Protocol-Version: 2025-06-18" \
            -d '{"method": "tools/call", "params": {"name": "rust_query", "arguments": {"query": "test"}}}' \
            "$BASE_URL/mcp" > /dev/null 2>&1 &
        
        wait $monitor_pid
        
        log_info "Resource monitoring data saved to $monitor_file"
        
        # Analyze resource usage
        local max_cpu
        local max_memory
        local avg_cpu
        local avg_memory
        
        max_cpu=$(tail -n +2 "$monitor_file" | cut -d',' -f2 | sort -nr | head -1)
        max_memory=$(tail -n +2 "$monitor_file" | cut -d',' -f3 | sort -nr | head -1)
        avg_cpu=$(tail -n +2 "$monitor_file" | cut -d',' -f2 | awk '{sum+=$1; count++} END {print sum/count}')
        avg_memory=$(tail -n +2 "$monitor_file" | cut -d',' -f3 | awk '{sum+=$1; count++} END {print sum/count}')
        
        log_info "Resource usage - CPU: avg=${avg_cpu}%, max=${max_cpu}% | Memory: avg=${avg_memory}MB, max=${max_memory}MB"
        
        if (( $(echo "$max_memory < 512" | bc -l) )); then
            log_success "Memory usage within limits (max: ${max_memory}MB < 512MB)"
        else
            log_warning "Memory usage high (max: ${max_memory}MB)"
        fi
    else
        log_warning "Could not find doc server process for resource monitoring"
    fi
}

# Database performance testing
benchmark_database() {
    log_info "Benchmarking database query performance..."
    
    # This test would typically connect directly to the database
    # For now, we'll test through the API as a proxy
    
    local db_results_file="$OUTPUT_DIR/database_$TIMESTAMP.json"
    echo "[]" > "$db_results_file"
    
    # Test different types of database operations through API
    local operations=(
        "vector_search:rust_query:test query for vector search"
        "metadata_filter:birdeye_query:api endpoint filtering"
        "full_text:solana_query:consensus algorithm documentation"
    )
    
    for operation in "${operations[@]}"; do
        IFS=':' read -ra PARTS <<< "$operation"
        local op_type="${PARTS[0]}"
        local tool="${PARTS[1]}"
        local query="${PARTS[2]}"
        
        log_info "Testing $op_type operation..."
        
        local times=()
        local count=5
        
        for i in $(seq 1 $count); do
            local start_time
            local end_time
            local duration
            
            start_time=$(date +%s.%N)
            curl -s -X POST "$BASE_URL/mcp" \
                -H "Content-Type: application/json" \
                -H "MCP-Protocol-Version: 2025-06-18" \
                -d "{\"method\": \"tools/call\", \"params\": {\"name\": \"$tool\", \"arguments\": {\"query\": \"$query\", \"limit\": 10}}}" \
                -o /dev/null
            end_time=$(date +%s.%N)
            
            duration=$(echo "$end_time - $start_time" | bc -l)
            times+=("$duration")
        done
        
        local avg_time
        avg_time=$(printf '%s\n' "${times[@]}" | awk '{sum+=$1; count++} END {print sum/count}')
        
        local result
        result=$(jq -n \
            --arg operation "$op_type" \
            --arg tool "$tool" \
            --argjson avg_time "$avg_time" \
            --argjson meets_requirement "$(echo "$avg_time < $MAX_RESPONSE_TIME" | bc -l)" \
            '{
                operation: $operation,
                tool: $tool,
                avg_query_time: $avg_time,
                meets_requirement: ($meets_requirement == 1)
            }')
        
        jq ". += [$result]" "$db_results_file" > "$db_results_file.tmp" && mv "$db_results_file.tmp" "$db_results_file"
        
        if (( $(echo "$avg_time < $MAX_RESPONSE_TIME" | bc -l) )); then
            log_success "$op_type: avg=${avg_time}s (meets requirement)"
        else
            log_error "$op_type: avg=${avg_time}s (exceeds requirement)"
        fi
    done
}

# Generate performance report
generate_performance_report() {
    log_info "Generating performance report..."
    
    local report_file="$OUTPUT_DIR/performance_report_$TIMESTAMP.md"
    
    cat > "$report_file" << EOF
# Performance Benchmark Report

**Generated:** $(date)
**Server:** $BASE_URL
**Test Duration:** ${BENCHMARK_DURATION}s
**Requirement:** < ${MAX_RESPONSE_TIME}s response time

## Response Time Analysis

EOF

    if [[ -f "$OUTPUT_DIR/response_times_$TIMESTAMP.json" ]]; then
        echo "### Query Performance by Complexity" >> "$report_file"
        echo "" >> "$report_file"
        
        jq -r '.[] | "- **\(.complexity)** (\(.tool)): avg=\(.avg_response_time)s, p95=\(.p95_response_time)s, success=\(.successful_requests)/\(.successful_requests + .failed_requests) [\(if .meets_requirement then "✅ PASS" else "❌ FAIL" end)]"' \
            "$OUTPUT_DIR/response_times_$TIMESTAMP.json" >> "$report_file"
        echo "" >> "$report_file"
    fi

    if [[ -f "$OUTPUT_DIR/throughput_$TIMESTAMP.txt" ]]; then
        echo "## Throughput Analysis" >> "$report_file"
        echo "" >> "$report_file"
        echo "\`\`\`" >> "$report_file"
        tail -20 "$OUTPUT_DIR/throughput_$TIMESTAMP.txt" >> "$report_file"
        echo "\`\`\`" >> "$report_file"
        echo "" >> "$report_file"
    fi

    if [[ -f "$OUTPUT_DIR/database_$TIMESTAMP.json" ]]; then
        echo "## Database Performance" >> "$report_file"
        echo "" >> "$report_file"
        
        jq -r '.[] | "- **\(.operation)**: avg=\(.avg_query_time)s [\(if .meets_requirement then "✅ PASS" else "❌ FAIL" end)]"' \
            "$OUTPUT_DIR/database_$TIMESTAMP.json" >> "$report_file"
        echo "" >> "$report_file"
    fi

    if [[ -f "$OUTPUT_DIR/resources_$TIMESTAMP.log" ]]; then
        echo "## Resource Utilization" >> "$report_file"
        echo "" >> "$report_file"
        echo "Resource usage data available in: \`resources_$TIMESTAMP.log\`" >> "$report_file"
        echo "" >> "$report_file"
    fi

    echo "## Summary" >> "$report_file"
    echo "" >> "$report_file"
    echo "All performance tests completed. Check individual test results above." >> "$report_file"
    
    log_info "Performance report generated: $report_file"
}

# Warmup function
warmup_server() {
    log_info "Warming up server with ${WARMUP_DURATION}s of requests..."
    
    for i in $(seq 1 "$WARMUP_DURATION"); do
        curl -s -X POST "$BASE_URL/mcp" \
            -H "Content-Type: application/json" \
            -H "MCP-Protocol-Version: 2025-06-18" \
            -d '{"method": "tools/list", "params": {}}' \
            -o /dev/null &
        sleep 1
    done
    
    wait
    log_info "Warmup completed"
}

# Main execution
main() {
    log_info "Starting performance benchmark suite..."
    log_info "Results will be saved to: $OUTPUT_DIR"
    echo
    
    # Check if server is running
    if ! curl -sf "$BASE_URL/health" > /dev/null; then
        log_error "Server not responding at $BASE_URL"
        exit 1
    fi
    
    # Warmup
    warmup_server
    
    # Run benchmarks
    benchmark_response_times
    benchmark_throughput
    benchmark_database
    monitor_resources
    
    # Generate report
    generate_performance_report
    
    log_success "Performance benchmarking completed!"
    log_info "Results directory: $OUTPUT_DIR"
}

# Check dependencies
for cmd in jq bc curl; do
    if ! command -v $cmd >/dev/null 2>&1; then
        log_error "$cmd is required but not installed"
        exit 1
    fi
done

# Run main function
main "$@"