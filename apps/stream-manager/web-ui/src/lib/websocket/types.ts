// WebSocket Event Types matching backend implementation

export enum EventType {
  StreamAdded = 'stream_added',
  StreamRemoved = 'stream_removed',
  StreamHealthChanged = 'stream_health_changed',
  RecordingStarted = 'recording_started',
  RecordingStopped = 'recording_stopped',
  StatisticsUpdate = 'statistics_update',
  SystemAlert = 'system_alert',
  ConfigChanged = 'config_changed',
  ErrorOccurred = 'error_occurred',
}

export interface WebSocketEvent<T = any> {
  id: string;
  timestamp: string; // ISO 8601 date string
  event_type: EventType;
  stream_id?: string;
  data: T;
}

export interface SubscriptionRequest {
  event_types?: EventType[];
  stream_ids?: string[];
}

// Specific event data types
export interface StreamAddedData {
  name: string;
  url: string;
  type: string;
  config?: Record<string, any>;
}

export interface StreamRemovedData {
  reason?: string;
}

export interface StreamHealthChangedData {
  health: 'healthy' | 'degraded' | 'unhealthy' | 'unknown';
  message?: string;
  metrics?: {
    dropped_frames?: number;
    bitrate?: number;
    latency_ms?: number;
  };
}

export interface RecordingStartedData {
  file_path: string;
  start_time: string;
  segment_duration?: number;
}

export interface RecordingStoppedData {
  file_path: string;
  stop_time: string;
  duration_seconds: number;
  file_size_bytes?: number;
}

export interface StatisticsUpdateData {
  streams: {
    [streamId: string]: {
      bitrate: number;
      fps: number;
      dropped_frames: number;
      total_frames: number;
      uptime_seconds: number;
    };
  };
  system: {
    cpu_usage_percent: number;
    memory_usage_mb: number;
    disk_usage_percent: number;
    gpu_usage_percent?: number;
  };
}

export interface SystemAlertData {
  message: string;
  level?: 'info' | 'warning' | 'error';
  client_id?: string;
  event_types?: EventType[];
  stream_ids?: string[];
}

export interface ConfigChangedData {
  section: string;
  changes: Record<string, any>;
}

export interface ErrorOccurredData {
  error: string;
  details?: string;
  stack_trace?: string;
  recoverable?: boolean;
}

// Type guards
export function isStreamAddedEvent(event: WebSocketEvent): event is WebSocketEvent<StreamAddedData> {
  return event.event_type === EventType.StreamAdded;
}

export function isStreamRemovedEvent(event: WebSocketEvent): event is WebSocketEvent<StreamRemovedData> {
  return event.event_type === EventType.StreamRemoved;
}

export function isStreamHealthChangedEvent(event: WebSocketEvent): event is WebSocketEvent<StreamHealthChangedData> {
  return event.event_type === EventType.StreamHealthChanged;
}

export function isRecordingStartedEvent(event: WebSocketEvent): event is WebSocketEvent<RecordingStartedData> {
  return event.event_type === EventType.RecordingStarted;
}

export function isRecordingStoppedEvent(event: WebSocketEvent): event is WebSocketEvent<RecordingStoppedData> {
  return event.event_type === EventType.RecordingStopped;
}

export function isStatisticsUpdateEvent(event: WebSocketEvent): event is WebSocketEvent<StatisticsUpdateData> {
  return event.event_type === EventType.StatisticsUpdate;
}

export function isSystemAlertEvent(event: WebSocketEvent): event is WebSocketEvent<SystemAlertData> {
  return event.event_type === EventType.SystemAlert;
}

export function isConfigChangedEvent(event: WebSocketEvent): event is WebSocketEvent<ConfigChangedData> {
  return event.event_type === EventType.ConfigChanged;
}

export function isErrorOccurredEvent(event: WebSocketEvent): event is WebSocketEvent<ErrorOccurredData> {
  return event.event_type === EventType.ErrorOccurred;
}

// Connection states
export enum ConnectionState {
  Connecting = 'connecting',
  Connected = 'connected',
  Disconnected = 'disconnected',
  Reconnecting = 'reconnecting',
  Error = 'error',
}

// WebSocket client configuration
export interface WebSocketConfig {
  url?: string;
  port?: number;
  path?: string;
  reconnect?: boolean;
  reconnectAttempts?: number;
  reconnectInterval?: number;
  reconnectDecay?: number;
  timeout?: number;
  debug?: boolean;
  heartbeatInterval?: number;
}

export const DEFAULT_CONFIG: WebSocketConfig = {
  port: 8080,
  path: '/ws',
  reconnect: true,
  reconnectAttempts: 10,
  reconnectInterval: 1000,
  reconnectDecay: 1.5,
  timeout: 5000,
  debug: false,
  heartbeatInterval: 30000,
};
