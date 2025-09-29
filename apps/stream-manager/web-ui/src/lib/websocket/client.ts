import { EventEmitter } from 'events';
import {
  WebSocketEvent,
  ConnectionState,
  WebSocketConfig,
  DEFAULT_CONFIG,
  EventType,
  SubscriptionRequest,
} from './types.ts';

export interface WebSocketClientEvents {
  open: () => void;
  close: (event: CloseEvent) => void;
  error: (error: Error) => void;
  message: (event: WebSocketEvent) => void;
  connectionStateChange: (state: ConnectionState) => void;
  reconnect: (attempt: number) => void;
}

export class WebSocketClient extends EventEmitter {
  private ws: WebSocket | null = null;
  private config: Required<WebSocketConfig>;
  private connectionState: ConnectionState = ConnectionState.Disconnected;
  private reconnectAttempt = 0;
  private reconnectTimer: number | null = null;
  private heartbeatTimer: number | null = null;
  private messageQueue: Array<string | ArrayBuffer> = [];
  private clientId: string | null = null;
  private isManualClose = false;
  private lastPongReceived = Date.now();
  private debugMode = false;

  constructor(config: WebSocketConfig = {}) {
    super();
    this.config = { ...DEFAULT_CONFIG, ...config } as Required<WebSocketConfig>;
    this.debugMode = this.config.debug;
  }

  // Connection management
  public connect(): void {
    if (this.ws && (this.ws.readyState === WebSocket.CONNECTING || this.ws.readyState === WebSocket.OPEN)) {
      this.debug('WebSocket is already connected or connecting');
      return;
    }

    this.isManualClose = false;
    this.setConnectionState(ConnectionState.Connecting);

    const wsUrl = this.buildWebSocketUrl();
    this.debug(`Connecting to WebSocket: ${wsUrl}`);

    try {
      this.ws = new WebSocket(wsUrl);
      this.setupEventHandlers();
      this.startConnectionTimeout();
    } catch (error) {
      this.debug(`Failed to create WebSocket: ${error}`);
      this.handleError(error as Error);
    }
  }

  public disconnect(): void {
    this.debug('Disconnecting WebSocket');
    this.isManualClose = true;
    this.cleanup();
  }

  public reconnect(): void {
    if (!this.config.reconnect || this.isManualClose) {
      return;
    }

    if (this.reconnectAttempt >= this.config.reconnectAttempts) {
      this.debug(`Max reconnection attempts reached (${this.config.reconnectAttempts})`);
      this.setConnectionState(ConnectionState.Error);
      this.emit('error', new Error('Max reconnection attempts reached'));
      return;
    }

    this.reconnectAttempt++;
    const delay = this.calculateReconnectDelay();

    this.debug(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempt}/${this.config.reconnectAttempts})`);
    this.setConnectionState(ConnectionState.Reconnecting);
    this.emit('reconnect', this.reconnectAttempt);

    this.reconnectTimer = window.setTimeout(() => {
      this.connect();
    }, delay);
  }

  // Message sending
  public send(message: string | ArrayBuffer): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(message);
      this.debug(`Sent message: ${typeof message === 'string' ? message : 'ArrayBuffer'}`);
    } else {
      this.debug('Queuing message (WebSocket not connected)');
      this.messageQueue.push(message);
    }
  }

  public sendJSON(data: any): void {
    this.send(JSON.stringify(data));
  }

  public subscribe(request: SubscriptionRequest): void {
    this.sendJSON(request);
  }

  // State management
  public getConnectionState(): ConnectionState {
    return this.connectionState;
  }

  public isConnected(): boolean {
    return this.connectionState === ConnectionState.Connected;
  }

  public getClientId(): string | null {
    return this.clientId;
  }

  // Event emitter type-safe wrappers
  public on<K extends keyof WebSocketClientEvents>(
    event: K,
    listener: WebSocketClientEvents[K]
  ): this {
    return super.on(event, listener);
  }

  public emit<K extends keyof WebSocketClientEvents>(
    event: K,
    ...args: Parameters<WebSocketClientEvents[K]>
  ): boolean {
    return super.emit(event, ...args);
  }

  public removeListener(event: string, listener: any): this {
    return super.removeListener(event, listener);
  }

  public removeAllListeners(event?: string): this {
    return super.removeAllListeners(event);
  }

  // Private methods
  private buildWebSocketUrl(): string {
    if (this.config.url) {
      return this.config.url;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.hostname;
    const port = this.config.port;
    const path = this.config.path;

    return `${protocol}//${host}:${port}${path}`;
  }

  private setupEventHandlers(): void {
    if (!this.ws) return;

    this.ws.onopen = this.handleOpen.bind(this);
    this.ws.onclose = this.handleClose.bind(this);
    this.ws.onerror = this.handleError.bind(this);
    this.ws.onmessage = this.handleMessage.bind(this);
  }

  private handleOpen(): void {
    this.debug('WebSocket connected');
    this.reconnectAttempt = 0;
    this.setConnectionState(ConnectionState.Connected);
    this.emit('open');
    this.flushMessageQueue();
    this.startHeartbeat();
  }

  private handleClose(event: CloseEvent): void {
    this.debug(`WebSocket closed: code=${event.code}, reason=${event.reason}`);
    this.cleanup();
    this.emit('close', event);

    if (!this.isManualClose) {
      this.reconnect();
    }
  }

  private handleError(error: Error | Event): void {
    const errorObj = error instanceof Error ? error : new Error('WebSocket error');
    this.debug(`WebSocket error: ${errorObj.message}`);
    this.emit('error', errorObj);
  }

  private handleMessage(event: MessageEvent): void {
    this.lastPongReceived = Date.now();

    try {
      const data = JSON.parse(event.data) as WebSocketEvent;
      this.debug(`Received message: ${data.event_type}`);

      // Handle system alerts for client ID and subscription confirmations
      if (data.event_type === EventType.SystemAlert && data.data) {
        const alertData = data.data as any;
        if (alertData.client_id) {
          this.clientId = alertData.client_id;
          this.debug(`Client ID assigned: ${this.clientId}`);
        }
      }

      this.emit('message', data);
    } catch (error) {
      this.debug(`Failed to parse message: ${error}`);
      this.emit('error', new Error(`Failed to parse WebSocket message: ${error}`));
    }
  }

  private setConnectionState(state: ConnectionState): void {
    if (this.connectionState !== state) {
      this.debug(`Connection state changed: ${this.connectionState} -> ${state}`);
      this.connectionState = state;
      this.emit('connectionStateChange', state);
    }
  }

  private cleanup(): void {
    this.stopHeartbeat();
    this.stopReconnectTimer();

    if (this.ws) {
      this.ws.onopen = null;
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.onmessage = null;

      if (this.ws.readyState === WebSocket.OPEN) {
        this.ws.close();
      }

      this.ws = null;
    }

    this.setConnectionState(ConnectionState.Disconnected);
  }

  private calculateReconnectDelay(): number {
    const baseDelay = this.config.reconnectInterval;
    const decay = this.config.reconnectDecay;
    return Math.min(baseDelay * Math.pow(decay, this.reconnectAttempt - 1), 30000);
  }

  private startConnectionTimeout(): void {
    window.setTimeout(() => {
      if (this.ws && this.ws.readyState === WebSocket.CONNECTING) {
        this.debug('Connection timeout');
        this.ws.close();
      }
    }, this.config.timeout);
  }

  private stopReconnectTimer(): void {
    if (this.reconnectTimer) {
      window.clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private flushMessageQueue(): void {
    while (this.messageQueue.length > 0 && this.ws && this.ws.readyState === WebSocket.OPEN) {
      const message = this.messageQueue.shift();
      if (message) {
        this.ws.send(message);
        this.debug('Sent queued message');
      }
    }
  }

  private startHeartbeat(): void {
    this.stopHeartbeat();

    if (!this.config.heartbeatInterval || this.config.heartbeatInterval <= 0) {
      return;
    }

    this.heartbeatTimer = window.setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        // Check if we've received a pong recently
        const timeSinceLastPong = Date.now() - this.lastPongReceived;
        if (timeSinceLastPong > this.config.heartbeatInterval * 2) {
          this.debug('Heartbeat timeout - no pong received');
          this.ws.close();
          return;
        }

        // Send ping
        try {
          // Send a ping frame (empty string as we're using text frames)
          this.ws.send('ping');
          this.debug('Sent heartbeat ping');
        } catch (error) {
          this.debug(`Failed to send heartbeat: ${error}`);
        }
      }
    }, this.config.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      window.clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private debug(message: string): void {
    if (this.debugMode) {
      console.log(`[WebSocketClient] ${message}`);
    }
  }
}

// Singleton instance
let defaultClient: WebSocketClient | null = null;

export function getDefaultWebSocketClient(config?: WebSocketConfig): WebSocketClient {
  if (!defaultClient) {
    defaultClient = new WebSocketClient(config);
  }
  return defaultClient;
}

export function resetDefaultWebSocketClient(): void {
  if (defaultClient) {
    defaultClient.disconnect();
    defaultClient = null;
  }
}