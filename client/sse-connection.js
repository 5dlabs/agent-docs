/**
 * SSE Connection Manager with Automatic Reconnection
 * 
 * This class provides a robust SSE (Server-Sent Events) connection with:
 * - Automatic reconnection with exponential backoff
 * - Message buffering during disconnections
 * - Connection state management
 * - Event handling and callbacks
 */

class SSEConnection {
    constructor(url, options = {}) {
        this.url = url;
        this.options = {
            heartbeatInterval: 30000,        // 30 seconds
            initialRetry: 1000,              // 1 second
            maxRetry: 60000,                 // 60 seconds  
            jitterMax: 500,                  // 500ms random jitter
            maxBufferSize: 1000,             // Max buffered messages
            reconnectAttempts: -1,           // Infinite reconnection attempts (-1)
            connectionTimeout: 90000,        // 90 seconds timeout
            ...options
        };
        
        this.state = 'disconnected';
        this.eventSource = null;
        this.retryDelay = this.options.initialRetry;
        this.reconnectCount = 0;
        this.messageBuffer = [];
        this.lastHeartbeat = null;
        this.heartbeatTimer = null;
        this.connectionId = null;
        
        // Event callbacks
        this.onOpen = null;
        this.onMessage = null;
        this.onError = null;
        this.onClose = null;
        this.onReconnecting = null;
        this.onBufferFlush = null;
        
        this.connect();
    }
    
    /**
     * Initiate connection to SSE endpoint
     */
    connect() {
        if (this.state === 'connecting') {
            console.log('SSE: Already connecting, ignoring duplicate connect request');
            return;
        }
        
        this.setState('connecting');
        console.log(`SSE: Connecting to ${this.url}...`);
        
        try {
            this.eventSource = new EventSource(this.url);
            this.setupEventHandlers();
            this.startHeartbeatTimer();
        } catch (error) {
            console.error('SSE: Failed to create EventSource:', error);
            this.handleConnectionError();
        }
    }
    
    /**
     * Setup event handlers for the EventSource
     */
    setupEventHandlers() {
        this.eventSource.onopen = (event) => {
            console.log('SSE: Connection established');
            this.setState('connected');
            this.retryDelay = this.options.initialRetry; // Reset retry delay
            this.reconnectCount = 0;
            this.lastHeartbeat = Date.now();
            
            // Flush buffered messages
            this.flushMessageBuffer();
            
            if (this.onOpen) this.onOpen(event);
        };
        
        this.eventSource.onmessage = (event) => {
            this.handleMessage(event);
        };
        
        this.eventSource.onerror = (event) => {
            console.error('SSE: Connection error occurred');
            this.handleConnectionError();
            if (this.onError) this.onError(event);
        };
        
        // Handle specific event types
        this.eventSource.addEventListener('connected', (event) => {
            const data = this.parseEventData(event.data);
            if (data && data.connection_id) {
                this.connectionId = data.connection_id;
                console.log(`SSE: Connection established with ID: ${this.connectionId}`);
            }
            this.handleMessage(event);
        });
        
        this.eventSource.addEventListener('heartbeat', (event) => {
            this.lastHeartbeat = Date.now();
            const data = this.parseEventData(event.data);
            if (data && data.timestamp) {
                console.debug(`SSE: Heartbeat received (seq: ${data.sequence || 'N/A'})`);
            }
            this.handleMessage(event);
        });
        
        this.eventSource.addEventListener('reconnecting', (event) => {
            console.log('SSE: Server acknowledged reconnection');
            this.handleMessage(event);
        });
    }
    
    /**
     * Handle incoming messages
     */
    handleMessage(event) {
        if (this.onMessage) {
            this.onMessage(event);
        }
    }
    
    /**
     * Parse event data safely
     */
    parseEventData(data) {
        try {
            return JSON.parse(data);
        } catch (error) {
            console.warn('SSE: Failed to parse event data:', data);
            return null;
        }
    }
    
    /**
     * Handle connection errors and initiate reconnection
     */
    handleConnectionError() {
        if (this.state === 'disconnected') {
            return; // Already handling disconnection
        }
        
        this.setState('reconnecting');
        this.cleanup();
        
        if (this.options.reconnectAttempts !== -1 && this.reconnectCount >= this.options.reconnectAttempts) {
            console.error('SSE: Maximum reconnection attempts exceeded');
            this.setState('failed');
            return;
        }
        
        this.reconnectCount++;
        const delay = this.getRetryDelay();
        
        console.log(`SSE: Reconnecting in ${delay}ms (attempt ${this.reconnectCount})`);
        
        if (this.onReconnecting) {
            this.onReconnecting({ 
                attempt: this.reconnectCount, 
                delay: delay,
                maxAttempts: this.options.reconnectAttempts 
            });
        }
        
        setTimeout(() => {
            if (this.state === 'reconnecting') {
                this.connect();
            }
        }, delay);
    }
    
    /**
     * Calculate retry delay with exponential backoff and jitter
     */
    getRetryDelay() {
        const jitter = Math.random() * this.options.jitterMax;
        const delay = Math.min(this.retryDelay + jitter, this.options.maxRetry);
        this.retryDelay = Math.min(this.retryDelay * 2, this.options.maxRetry);
        return Math.floor(delay);
    }
    
    /**
     * Start heartbeat monitoring timer
     */
    startHeartbeatTimer() {
        if (this.heartbeatTimer) {
            clearInterval(this.heartbeatTimer);
        }
        
        this.heartbeatTimer = setInterval(() => {
            if (this.state === 'connected' && this.lastHeartbeat) {
                const timeSinceLastHeartbeat = Date.now() - this.lastHeartbeat;
                const heartbeatTimeout = this.options.heartbeatInterval * 3; // Allow 3 missed heartbeats
                
                if (timeSinceLastHeartbeat > heartbeatTimeout) {
                    console.warn(`SSE: No heartbeat for ${timeSinceLastHeartbeat}ms, considering connection lost`);
                    this.handleConnectionError();
                }
            }
        }, this.options.heartbeatInterval);
    }
    
    /**
     * Add message to buffer during disconnection
     */
    bufferMessage(message) {
        if (this.messageBuffer.length >= this.options.maxBufferSize) {
            console.warn('SSE: Message buffer full, dropping oldest message');
            this.messageBuffer.shift(); // Remove oldest message (FIFO)
        }
        
        this.messageBuffer.push({
            message: message,
            timestamp: Date.now()
        });
        
        console.debug(`SSE: Message buffered. Buffer size: ${this.messageBuffer.length}`);
    }
    
    /**
     * Flush buffered messages after reconnection
     */
    flushMessageBuffer() {
        if (this.messageBuffer.length === 0) {
            return;
        }
        
        console.log(`SSE: Flushing ${this.messageBuffer.length} buffered messages`);
        
        const bufferedMessages = [...this.messageBuffer];
        this.messageBuffer = [];
        
        if (this.onBufferFlush) {
            this.onBufferFlush(bufferedMessages);
        }
        
        // Process buffered messages
        bufferedMessages.forEach(({ message, timestamp }) => {
            console.debug(`SSE: Processing buffered message from ${new Date(timestamp).toISOString()}`);
            // Note: In a real implementation, you might want to replay these messages
            // through the normal message handling pipeline
        });
    }
    
    /**
     * Set connection state and notify listeners
     */
    setState(newState) {
        if (this.state !== newState) {
            console.log(`SSE: State changed from ${this.state} to ${newState}`);
            this.state = newState;
        }
    }
    
    /**
     * Send a message (for future bidirectional communication)
     * Note: SSE is unidirectional, but this could be extended for WebSockets
     */
    send(message) {
        if (this.state !== 'connected') {
            console.warn('SSE: Cannot send message, connection not established. Buffering message.');
            this.bufferMessage(message);
            return false;
        }
        
        // SSE doesn't support client-to-server messages natively
        // This would need to be implemented via separate HTTP requests
        console.warn('SSE: Sending messages not supported in SSE protocol');
        return false;
    }
    
    /**
     * Disconnect and cleanup
     */
    disconnect() {
        console.log('SSE: Disconnecting...');
        this.setState('disconnected');
        this.cleanup();
    }
    
    /**
     * Cleanup resources
     */
    cleanup() {
        if (this.eventSource) {
            this.eventSource.close();
            this.eventSource = null;
        }
        
        if (this.heartbeatTimer) {
            clearInterval(this.heartbeatTimer);
            this.heartbeatTimer = null;
        }
    }
    
    /**
     * Get connection statistics
     */
    getStats() {
        return {
            state: this.state,
            connectionId: this.connectionId,
            reconnectCount: this.reconnectCount,
            bufferedMessages: this.messageBuffer.length,
            lastHeartbeat: this.lastHeartbeat ? new Date(this.lastHeartbeat) : null,
            retryDelay: this.retryDelay,
            url: this.url
        };
    }
    
    /**
     * Check if connection is healthy
     */
    isHealthy() {
        if (this.state !== 'connected') {
            return false;
        }
        
        if (!this.lastHeartbeat) {
            return false;
        }
        
        const timeSinceLastHeartbeat = Date.now() - this.lastHeartbeat;
        return timeSinceLastHeartbeat < (this.options.heartbeatInterval * 2);
    }
}

// Export for use in different module systems
if (typeof module !== 'undefined' && module.exports) {
    module.exports = SSEConnection;
} else if (typeof define === 'function' && define.amd) {
    define([], function() { return SSEConnection; });
} else {
    window.SSEConnection = SSEConnection;
}