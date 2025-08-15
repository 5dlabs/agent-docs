#!/usr/bin/env node

/**
 * SSE Load Testing Script
 *
 * This script tests the SSE endpoint with multiple concurrent connections
 * to validate performance, reliability, and resource usage.
 */

const EventSource = require("eventsource");

class SSELoadTester {
  constructor(options = {}) {
    this.url = options.url || "http://localhost:3001/sse";
    this.connections = options.connections || 10;
    this.duration = options.duration || 60; // seconds
    this.reportInterval = options.reportInterval || 5; // seconds

    this.activeConnections = new Map();
    this.stats = {
      connectionsEstablished: 0,
      connectionsFailed: 0,
      messagesReceived: 0,
      heartbeatsReceived: 0,
      reconnections: 0,
      errors: 0,
      startTime: null,
      endTime: null,
    };

    this.isRunning = false;
    this.reportTimer = null;
  }

  async start() {
    console.log(
      `Starting SSE load test with ${this.connections} connections for ${this.duration} seconds`,
    );
    console.log(`Target URL: ${this.url}`);
    console.log("─".repeat(80));

    this.stats.startTime = Date.now();
    this.isRunning = true;

    // Start periodic reporting
    this.reportTimer = setInterval(() => {
      this.printStats();
    }, this.reportInterval * 1000);

    // Create connections
    for (let i = 0; i < this.connections; i++) {
      this.createConnection(i);
      // Stagger connection creation to avoid overwhelming the server
      await this.sleep(10);
    }

    // Run for specified duration
    await this.sleep(this.duration * 1000);

    // Stop test
    await this.stop();
  }

  createConnection(id) {
    const connectionInfo = {
      id: id,
      eventSource: null,
      connected: false,
      lastMessage: null,
      messagesReceived: 0,
      heartbeatsReceived: 0,
      reconnectCount: 0,
      startTime: Date.now(),
    };

    const eventSource = new EventSource(this.url);
    connectionInfo.eventSource = eventSource;

    eventSource.onopen = () => {
      connectionInfo.connected = true;
      this.stats.connectionsEstablished++;
      console.log(`Connection ${id}: Established`);
    };

    eventSource.onmessage = (event) => {
      connectionInfo.lastMessage = Date.now();
      connectionInfo.messagesReceived++;
      this.stats.messagesReceived++;

      try {
        const data = JSON.parse(event.data);
        if (data.type === "heartbeat") {
          connectionInfo.heartbeatsReceived++;
          this.stats.heartbeatsReceived++;
        }
      } catch (error) {
        // Not JSON, still count as message
      }
    };

    eventSource.onerror = (error) => {
      this.stats.errors++;

      if (connectionInfo.connected) {
        // This is a reconnection attempt
        connectionInfo.reconnectCount++;
        this.stats.reconnections++;
        console.log(
          `Connection ${id}: Reconnecting (attempt #${connectionInfo.reconnectCount})`,
        );
      } else {
        // Initial connection failed
        this.stats.connectionsFailed++;
        console.log(`Connection ${id}: Failed to connect`);
      }

      connectionInfo.connected = false;
    };

    // Handle specific event types
    eventSource.addEventListener("connected", (event) => {
      try {
        const data = JSON.parse(event.data);
        console.log(
          `Connection ${id}: Connected with ID ${data.connection_id}`,
        );
      } catch (error) {
        console.log(
          `Connection ${id}: Connected (could not parse connection data)`,
        );
      }
    });

    eventSource.addEventListener("heartbeat", (event) => {
      try {
        const data = JSON.parse(event.data);
        console.debug(`Connection ${id}: Heartbeat #${data.sequence || "N/A"}`);
      } catch (error) {
        // Heartbeat parsing failed, but still counted
      }
    });

    this.activeConnections.set(id, connectionInfo);
  }

  async stop() {
    console.log("Stopping load test...");
    this.isRunning = false;

    if (this.reportTimer) {
      clearInterval(this.reportTimer);
    }

    // Close all connections
    for (const [id, info] of this.activeConnections) {
      if (info.eventSource) {
        info.eventSource.close();
      }
    }

    this.stats.endTime = Date.now();

    // Final report
    console.log("\n" + "=".repeat(80));
    console.log("LOAD TEST COMPLETED");
    console.log("=".repeat(80));
    this.printFinalReport();
  }

  printStats() {
    const now = Date.now();
    const elapsed = (now - this.stats.startTime) / 1000;

    const activeConnections = Array.from(
      this.activeConnections.values(),
    ).filter((conn) => conn.connected).length;

    const messagesPerSecond = Math.round(this.stats.messagesReceived / elapsed);
    const heartbeatsPerSecond = Math.round(
      this.stats.heartbeatsReceived / elapsed,
    );

    console.log(
      `\n[${new Date().toLocaleTimeString()}] Stats (${elapsed.toFixed(1)}s elapsed):`,
    );
    console.log(
      `  Active Connections: ${activeConnections}/${this.connections}`,
    );
    console.log(
      `  Messages Received: ${this.stats.messagesReceived} (${messagesPerSecond}/s)`,
    );
    console.log(
      `  Heartbeats: ${this.stats.heartbeatsReceived} (${heartbeatsPerSecond}/s)`,
    );
    console.log(`  Reconnections: ${this.stats.reconnections}`);
    console.log(`  Errors: ${this.stats.errors}`);

    // Connection health check
    const unhealthyConnections = this.checkConnectionHealth();
    if (unhealthyConnections.length > 0) {
      console.log(
        `  ⚠️  Unhealthy connections: ${unhealthyConnections.length}`,
      );
    }
  }

  printFinalReport() {
    const duration = (this.stats.endTime - this.stats.startTime) / 1000;
    const successRate =
      (this.stats.connectionsEstablished /
        (this.stats.connectionsEstablished + this.stats.connectionsFailed)) *
      100;

    console.log(`Duration: ${duration.toFixed(1)} seconds`);
    console.log(`Target Connections: ${this.connections}`);
    console.log(
      `Connections Established: ${this.stats.connectionsEstablished}`,
    );
    console.log(`Connection Failures: ${this.stats.connectionsFailed}`);
    console.log(`Success Rate: ${successRate.toFixed(1)}%`);
    console.log(`Total Messages Received: ${this.stats.messagesReceived}`);
    console.log(`Total Heartbeats: ${this.stats.heartbeatsReceived}`);
    console.log(
      `Messages per Second (avg): ${Math.round(this.stats.messagesReceived / duration)}`,
    );
    console.log(
      `Heartbeats per Second (avg): ${Math.round(this.stats.heartbeatsReceived / duration)}`,
    );
    console.log(`Total Reconnections: ${this.stats.reconnections}`);
    console.log(`Total Errors: ${this.stats.errors}`);

    // Connection analysis
    console.log("\nConnection Analysis:");
    this.analyzeConnections();

    // Performance assessment
    console.log("\nPerformance Assessment:");
    this.assessPerformance(duration, successRate);
  }

  analyzeConnections() {
    const now = Date.now();
    let totalMessages = 0;
    let totalHeartbeats = 0;
    let totalReconnects = 0;

    for (const [id, info] of this.activeConnections) {
      totalMessages += info.messagesReceived;
      totalHeartbeats += info.heartbeatsReceived;
      totalReconnects += info.reconnectCount;

      const duration = (now - info.startTime) / 1000;
      const messageRate = Math.round(info.messagesReceived / duration);

      console.log(
        `  Connection ${id}: ${info.messagesReceived} msgs (${messageRate}/s), ` +
          `${info.heartbeatsReceived} heartbeats, ${info.reconnectCount} reconnects`,
      );
    }

    const avgMessages = Math.round(totalMessages / this.activeConnections.size);
    const avgHeartbeats = Math.round(
      totalHeartbeats / this.activeConnections.size,
    );
    const avgReconnects =
      Math.round((totalReconnects / this.activeConnections.size) * 100) / 100;

    console.log(
      `  Average per connection: ${avgMessages} msgs, ${avgHeartbeats} heartbeats, ${avgReconnects} reconnects`,
    );
  }

  assessPerformance(duration, successRate) {
    const expectedHeartbeats =
      Math.floor(duration / 30) * this.stats.connectionsEstablished; // 30s interval
    const heartbeatDeliveryRate =
      (this.stats.heartbeatsReceived / expectedHeartbeats) * 100;

    console.log(`Expected heartbeats: ~${expectedHeartbeats}`);
    console.log(
      `Heartbeat delivery rate: ${heartbeatDeliveryRate.toFixed(1)}%`,
    );

    // Performance criteria
    const criteria = {
      connectionSuccess: successRate >= 95,
      heartbeatReliability: heartbeatDeliveryRate >= 95,
      lowReconnectionRate:
        this.stats.reconnections / this.stats.connectionsEstablished < 0.1,
      lowErrorRate: this.stats.errors / this.stats.messagesReceived < 0.01,
    };

    console.log("\nPerformance Criteria:");
    console.log(
      `✓ Connection Success Rate >= 95%: ${criteria.connectionSuccess ? "PASS" : "FAIL"} (${successRate.toFixed(1)}%)`,
    );
    console.log(
      `✓ Heartbeat Delivery Rate >= 95%: ${criteria.heartbeatReliability ? "PASS" : "FAIL"} (${heartbeatDeliveryRate.toFixed(1)}%)`,
    );
    console.log(
      `✓ Low Reconnection Rate < 10%: ${criteria.lowReconnectionRate ? "PASS" : "FAIL"}`,
    );
    console.log(
      `✓ Low Error Rate < 1%: ${criteria.lowErrorRate ? "PASS" : "FAIL"}`,
    );

    const overallPass = Object.values(criteria).every((result) => result);
    console.log(`\n${overallPass ? "🎉 OVERALL: PASS" : "❌ OVERALL: FAIL"}`);
  }

  checkConnectionHealth() {
    const now = Date.now();
    const unhealthy = [];

    for (const [id, info] of this.activeConnections) {
      if (!info.connected) {
        unhealthy.push({ id, reason: "disconnected" });
      } else if (info.lastMessage && now - info.lastMessage > 90000) {
        // 90s timeout
        unhealthy.push({ id, reason: "no_recent_messages" });
      }
    }

    return unhealthy;
  }

  sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

// CLI Interface
function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    url: "http://localhost:3001/sse",
    connections: 10,
    duration: 60,
    reportInterval: 5,
  };

  for (let i = 0; i < args.length; i += 2) {
    const key = args[i];
    const value = args[i + 1];

    switch (key) {
      case "--url":
        options.url = value;
        break;
      case "--connections":
        options.connections = parseInt(value);
        break;
      case "--duration":
        options.duration = parseInt(value);
        break;
      case "--report-interval":
        options.reportInterval = parseInt(value);
        break;
      case "--help":
        printHelp();
        process.exit(0);
      default:
        console.error(`Unknown option: ${key}`);
        process.exit(1);
    }
  }

  return options;
}

function printHelp() {
  console.log(`
SSE Load Testing Tool

Usage: node load_test_sse.js [options]

Options:
  --url <url>              SSE endpoint URL (default: http://localhost:3001/sse)
  --connections <number>   Number of concurrent connections (default: 10)
  --duration <seconds>     Test duration in seconds (default: 60)
  --report-interval <sec>  Reporting interval in seconds (default: 5)
  --help                   Show this help message

Examples:
  node load_test_sse.js --connections 50 --duration 120
  node load_test_sse.js --url http://example.com/sse --connections 100
`);
}

// Main execution
if (require.main === module) {
  const options = parseArgs();

  console.log("SSE Load Testing Tool");
  console.log("====================\n");

  const tester = new SSELoadTester(options);

  // Handle graceful shutdown
  process.on("SIGINT", async () => {
    console.log("\nReceived SIGINT, stopping test...");
    await tester.stop();
    process.exit(0);
  });

  tester.start().catch((error) => {
    console.error("Load test failed:", error);
    process.exit(1);
  });
}

module.exports = SSELoadTester;
