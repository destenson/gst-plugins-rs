// API Response Types

// Health & Status
export interface HealthResponse {
  status: string;
  uptime: number;
  version: string;
}

export interface SystemStatus {
  active_streams: number;
  total_streams: number;
  recording_streams: number;
  cpu_usage: number;
  memory_usage: number;
  disk_usage: {
    used: number;
    total: number;
  };
}

// Stream Types
export interface StreamRecording {
  enabled: boolean;
  status?: 'recording' | 'paused' | 'stopped';
  current_file?: string;
  duration?: number;
  total_size?: number;
  segment_duration?: number;
  retention_days?: number;
  segments?: RecordingSegment[];
}

export interface RecordingSegment {
  filename: string;
  size: number;
  duration: number;
}

export interface StreamMetrics {
  bitrate: number;
  framerate: number;
  resolution?: string;
  packets_received: number;
  packets_lost: number;
}

export interface DetailedMetrics {
  bitrate: {
    current: number;
    average: number;
    peak: number;
  };
  framerate: {
    current: number;
    average: number;
    dropped: number;
  };
  latency: {
    pipeline: number;
    network: number;
    processing: number;
  };
  packets: {
    received: number;
    lost: number;
    recovered: number;
  };
  errors: {
    decode: number;
    network: number;
    total: number;
  };
}

export interface StreamInference {
  enabled: boolean;
  model?: string;
  threshold?: number;
}

export interface StreamReconnect {
  enabled: boolean;
  max_attempts?: number;
  backoff_ms?: number;
}

export interface StreamPipeline {
  state: 'playing' | 'paused' | 'stopped' | 'null';
  latency: number;
  buffer_level: number;
}

export interface Stream {
  id: string;
  source_url: string;
  status: 'active' | 'inactive' | 'error' | 'initializing' | 'starting' | 'stopping' | 'restarting';
  created_at?: string;
  last_connected?: string;
  recording?: StreamRecording;
  metrics?: StreamMetrics;
  inference?: StreamInference;
  reconnect?: StreamReconnect;
  pipeline?: StreamPipeline;
  errors?: string[];
}

export interface StreamListResponse {
  streams: Stream[];
}

export interface CreateStreamRequest {
  id: string;
  source_url: string;
  recording?: Partial<StreamRecording>;
  inference?: StreamInference;
  reconnect?: StreamReconnect;
}

export interface UpdateStreamRequest {
  recording?: Partial<StreamRecording>;
  inference?: Partial<StreamInference>;
  reconnect?: Partial<StreamReconnect>;
}

// Recording Types
export interface StartRecordingRequest {
  filename?: string;
  segment_duration?: number;
}

export interface StartRecordingResponse {
  status: string;
  filename: string;
}

export interface StopRecordingResponse {
  status: string;
  files: string[];
  total_duration: number;
  total_size: number;
}

export interface Recording {
  filename: string;
  path: string;
  size: number;
  duration: number;
  created_at: string;
}

export interface RecordingListResponse {
  recordings: Recording[];
  total_size: number;
  total_duration: number;
}

// Configuration Types
export interface ServerConfig {
  port: number;
  host: string;
}

export interface RecordingConfig {
  base_path: string;
  segment_duration: number;
  retention_days: number;
}

export interface InferenceConfig {
  enabled: boolean;
  device: 'cpu' | 'gpu';
  models_path: string;
}

export interface SystemConfig {
  server: ServerConfig;
  recording: RecordingConfig;
  inference: InferenceConfig;
}

export interface UpdateConfigRequest {
  server?: Partial<ServerConfig>;
  recording?: Partial<RecordingConfig>;
  inference?: Partial<InferenceConfig>;
}


// WebSocket Event Types
export type EventType =
  | 'stream.started'
  | 'stream.stopped'
  | 'stream.error'
  | 'recording.started'
  | 'recording.stopped'
  | 'recording.segment_created'
  | 'metrics.update'
  | 'config.reloaded'
  | 'system.warning'
  | 'system.error';

export interface WebSocketEvent {
  type: EventType;
  stream_id?: string;
  filename?: string;
  error?: string;
  metrics?: Partial<StreamMetrics>;
  timestamp: string;
  [key: string]: any;
}

// Error Types
export interface APIError {
  error: {
    code: string;
    message: string;
    details?: Record<string, any>;
  };
}

export type ErrorCode =
  | 'STREAM_NOT_FOUND'
  | 'STREAM_ALREADY_EXISTS'
  | 'INVALID_CONFIGURATION'
  | 'STREAM_BUSY'
  | 'RECORDING_IN_PROGRESS'
  | 'NO_RECORDING'
  | 'UNAUTHORIZED'
  | 'INTERNAL_ERROR';

// Query Parameters
export interface StreamListQuery {
  status?: 'active' | 'inactive' | 'error';
  recording?: boolean;
}

export interface RecordingListQuery {
  start_date?: string;
  end_date?: string;
  limit?: number;
}

// Response helpers
export type APIResponse<T> = T | APIError;

export function isAPIError(response: any): response is APIError {
  return response && typeof response === 'object' && 'error' in response;
}