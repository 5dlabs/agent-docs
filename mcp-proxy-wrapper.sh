#!/bin/bash
# MCP Proxy Wrapper for adding X-Client-Id header
# This script acts as a proxy to add the required header for session stability

# Configuration
DOC_SERVER_URL="${DOC_SERVER_URL:-http://doc-server-agent-docs-server.mcp.svc.cluster.local:80/mcp}"
CLIENT_ID="${MCP_CLIENT_ID:-cursor-$(hostname)-$(whoami)}"

# Create a simple Node.js proxy server
cat > /tmp/mcp-proxy-$$.js << 'EOF'
const http = require('http');
const https = require('https');
const url = require('url');

const TARGET_URL = process.env.DOC_SERVER_URL;
const CLIENT_ID = process.env.CLIENT_ID;

console.error(`MCP Proxy starting - Target: ${TARGET_URL}, Client-Id: ${CLIENT_ID}`);

// Parse target URL
const targetUrl = new url.URL(TARGET_URL);
const isHttps = targetUrl.protocol === 'https:';
const httpModule = isHttps ? https : http;

// Create proxy server
const server = http.createServer((req, res) => {
    // Log incoming request
    console.error(`Proxy request: ${req.method} ${req.url}`);
    
    // Prepare proxy request options
    const options = {
        hostname: targetUrl.hostname,
        port: targetUrl.port || (isHttps ? 443 : 80),
        path: targetUrl.pathname + (req.url === '/' ? '' : req.url),
        method: req.method,
        headers: {
            ...req.headers,
            'X-Client-Id': CLIENT_ID,
            'host': targetUrl.host
        }
    };
    
    // Make proxy request
    const proxyReq = httpModule.request(options, (proxyRes) => {
        console.error(`Proxy response: ${proxyRes.statusCode}`);
        
        // Forward status and headers
        res.writeHead(proxyRes.statusCode, proxyRes.headers);
        
        // Pipe response
        proxyRes.pipe(res);
    });
    
    proxyReq.on('error', (err) => {
        console.error('Proxy error:', err);
        res.writeHead(502);
        res.end('Proxy Error');
    });
    
    // Pipe request body if present
    req.pipe(proxyReq);
});

// Start server on random available port
server.listen(0, '127.0.0.1', () => {
    const port = server.address().port;
    console.log(`http://127.0.0.1:${port}/mcp`);
});

// Handle cleanup
process.on('SIGTERM', () => {
    server.close(() => {
        process.exit(0);
    });
});
EOF

# Run the proxy and output the URL for Cursor to use
DOC_SERVER_URL="$DOC_SERVER_URL" CLIENT_ID="$CLIENT_ID" node /tmp/mcp-proxy-$$.js