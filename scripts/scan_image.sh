#!/bin/bash
# Security scanning script for container images using Trivy
# This script performs vulnerability scanning, SBOM generation, and compliance checks

set -e

# Configuration
IMAGE_NAME=${1:-"doc-server:latest"}
OUTPUT_DIR=${2:-"./security-reports"}
SEVERITY_THRESHOLD="HIGH,CRITICAL"
EXIT_ON_VIOLATION=${EXIT_ON_VIOLATION:-1}

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo "🔍 Starting security scan for image: $IMAGE_NAME"
echo "📁 Output directory: $OUTPUT_DIR"
echo "🚨 Severity threshold: $SEVERITY_THRESHOLD"

# Function to check if Trivy is installed
check_trivy() {
    if ! command -v trivy &> /dev/null; then
        echo "❌ Trivy is not installed. Please install Trivy first:"
        echo "   curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin"
        exit 1
    fi
    echo "✅ Trivy found: $(trivy version | head -n1)"
}

# Function to update Trivy database
update_trivy_db() {
    echo "🔄 Updating Trivy vulnerability database..."
    trivy image --download-db-only
}

# Function to perform vulnerability scan
vulnerability_scan() {
    echo "🛡️  Performing vulnerability scan..."
    
    # Scan with exit code enforcement for CRITICAL and HIGH vulnerabilities
    if [ "$EXIT_ON_VIOLATION" = "1" ]; then
        echo "🚨 Scanning with exit-code enforcement for $SEVERITY_THRESHOLD vulnerabilities"
        trivy image \
            --exit-code 1 \
            --severity "$SEVERITY_THRESHOLD" \
            --format table \
            "$IMAGE_NAME"
    else
        echo "ℹ️  Scanning without exit-code enforcement (informational mode)"
        trivy image \
            --severity "$SEVERITY_THRESHOLD" \
            --format table \
            "$IMAGE_NAME"
    fi
    
    # Generate detailed JSON report
    echo "📊 Generating detailed JSON report..."
    trivy image \
        --format json \
        --output "$OUTPUT_DIR/vulnerability-report.json" \
        "$IMAGE_NAME"
    
    # Generate SARIF format for CI/CD tooling integration
    echo "🔧 Generating SARIF report for CI/CD integration..."
    trivy image \
        --format sarif \
        --output "$OUTPUT_DIR/results.sarif" \
        "$IMAGE_NAME"
    
    echo "✅ Vulnerability scan complete"
}

# Function to generate SBOM (Software Bill of Materials)
generate_sbom() {
    echo "📋 Generating Software Bill of Materials (SBOM)..."
    
    # Generate SPDX-JSON SBOM
    trivy image \
        --format spdx-json \
        --output "$OUTPUT_DIR/sbom.spdx.json" \
        "$IMAGE_NAME"
    
    # Generate CycloneDX SBOM for broader compatibility
    trivy image \
        --format cyclonedx \
        --output "$OUTPUT_DIR/sbom.cyclonedx.json" \
        "$IMAGE_NAME"
    
    echo "✅ SBOM generation complete"
}

# Function to perform configuration scanning
config_scan() {
    echo "⚙️  Performing container configuration scan..."
    
    trivy config \
        --format table \
        --exit-code 0 \
        .
    
    # Generate configuration scan report
    trivy config \
        --format json \
        --output "$OUTPUT_DIR/config-scan.json" \
        .
    
    echo "✅ Configuration scan complete"
}

# Function to generate summary report
generate_summary() {
    echo "📈 Generating scan summary..."
    
    # Extract key metrics from the JSON report
    if [ -f "$OUTPUT_DIR/vulnerability-report.json" ]; then
        cat > "$OUTPUT_DIR/scan-summary.txt" <<EOF
Container Security Scan Summary
===============================

Image: $IMAGE_NAME
Scan Date: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
Trivy Version: $(trivy version | head -n1)

Vulnerability Summary:
$(jq -r '
if .Results then
  .Results[] | 
  if .Vulnerabilities then
    "Total Vulnerabilities: " + (.Vulnerabilities | length | tostring) + "\n" +
    "CRITICAL: " + ([.Vulnerabilities[] | select(.Severity == "CRITICAL")] | length | tostring) + "\n" +
    "HIGH: " + ([.Vulnerabilities[] | select(.Severity == "HIGH")] | length | tostring) + "\n" +
    "MEDIUM: " + ([.Vulnerabilities[] | select(.Severity == "MEDIUM")] | length | tostring) + "\n" +
    "LOW: " + ([.Vulnerabilities[] | select(.Severity == "LOW")] | length | tostring)
  else
    "No vulnerabilities found"
  end
else
  "No results available"
end
' "$OUTPUT_DIR/vulnerability-report.json" 2>/dev/null || echo "Unable to parse vulnerability report")

Files Generated:
- vulnerability-report.json (Detailed vulnerability report)
- vulnerability-report.sarif (SARIF format for CI/CD)
- sbom.spdx.json (SPDX Software Bill of Materials)
- sbom.cyclonedx.json (CycloneDX Software Bill of Materials)
- config-scan.json (Container configuration analysis)
- scan-summary.txt (This summary)

EOF
        echo "✅ Summary report generated"
        echo ""
        echo "📊 Scan Summary:"
        cat "$OUTPUT_DIR/scan-summary.txt"
    fi
}

# Function to cleanup on exit
cleanup() {
    if [ $? -ne 0 ]; then
        echo "❌ Security scan failed or was interrupted"
    fi
}

# Set up cleanup trap
trap cleanup EXIT

# Main execution
main() {
    echo "🚀 Starting container security scan pipeline..."
    
    check_trivy
    update_trivy_db
    vulnerability_scan
    generate_sbom
    config_scan
    generate_summary
    
    echo ""
    echo "🎉 Security scan pipeline completed successfully!"
    echo "📁 Results available in: $OUTPUT_DIR"
    
    # Final security assessment
    if [ -f "$OUTPUT_DIR/vulnerability-report.json" ]; then
        CRITICAL_COUNT=$(jq -r '[.Results[]? | .Vulnerabilities[]? | select(.Severity == "CRITICAL")] | length' "$OUTPUT_DIR/vulnerability-report.json" 2>/dev/null || echo "0")
        HIGH_COUNT=$(jq -r '[.Results[]? | .Vulnerabilities[]? | select(.Severity == "HIGH")] | length' "$OUTPUT_DIR/vulnerability-report.json" 2>/dev/null || echo "0")
        
        if [ "$CRITICAL_COUNT" -gt 0 ] || [ "$HIGH_COUNT" -gt 0 ]; then
            echo "❌ SECURITY VIOLATION: Found $CRITICAL_COUNT CRITICAL and $HIGH_COUNT HIGH severity vulnerabilities"
            if [ "$EXIT_ON_VIOLATION" = "1" ]; then
                echo "🚨 Failing build due to security policy violation"
                exit 1
            fi
        else
            echo "✅ SECURITY PASSED: No CRITICAL or HIGH severity vulnerabilities found"
        fi
    fi
}

# Run main function
main