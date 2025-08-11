/**
 * TypeScript definitions for SSEConnection class
 */

export interface SSEConnectionOptions {
    /** Heartbeat interval in milliseconds (default: 30000) */
    heartbeatInterval?: number;
    
    /** Initial retry delay in milliseconds (default: 1000) */
    initialRetry?: number;
    
    /** Maximum retry delay in milliseconds (default: 60000) */
    maxRetry?: number;
    
    /** Maximum jitter in milliseconds (default: 500) */
    jitterMax?: number;
    
    /** Maximum buffer size for messages (default: 1000) */
    maxBufferSize?: number;
    
    /** Maximum reconnection attempts, -1 for infinite (default: -1) */
    reconnectAttempts?: number;
    
    /** Connection timeout in milliseconds (default: 90000) */
    connectionTimeout?: number;
}

export interface BufferedMessage {
    message: any;
    timestamp: number;
}

export interface ReconnectionInfo {
    attempt: number;
    delay: number;
    maxAttempts: number;
}

export interface ConnectionStats {
    state: 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'failed';
    connectionId: string | null;
    reconnectCount: number;
    bufferedMessages: number;
    lastHeartbeat: Date | null;
    retryDelay: number;
    url: string;
}

export declare class SSEConnection {
    public readonly url: string;
    public readonly options: Required<SSEConnectionOptions>;
    public state: 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'failed';
    public connectionId: string | null;
    public reconnectCount: number;
    
    /** Callback for connection opened event */
    public onOpen: ((event: Event) => void) | null;
    
    /** Callback for message received event */
    public onMessage: ((event: MessageEvent) => void) | null;
    
    /** Callback for error event */
    public onError: ((event: Event) => void) | null;
    
    /** Callback for connection closed event */
    public onClose: ((event: Event) => void) | null;
    
    /** Callback for reconnecting event */
    public onReconnecting: ((info: ReconnectionInfo) => void) | null;
    
    /** Callback for buffer flush event */
    public onBufferFlush: ((messages: BufferedMessage[]) => void) | null;
    
    constructor(url: string, options?: SSEConnectionOptions);
    
    /** Initiate connection to SSE endpoint */
    public connect(): void;
    
    /** Send a message (buffered if not connected) */
    public send(message: any): boolean;
    
    /** Disconnect and cleanup */
    public disconnect(): void;
    
    /** Get connection statistics */
    public getStats(): ConnectionStats;
    
    /** Check if connection is healthy */
    public isHealthy(): boolean;
}

export default SSEConnection;