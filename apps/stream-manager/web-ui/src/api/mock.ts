import type {
  Stream,
  StreamListResponse,
  HealthResponse,
  SystemStatus,
  SystemConfig,
  DetailedMetrics,
  RecordingListResponse
} from './types/index.ts';

// Mock data generators
const generateMockStream = (id: string, index: number): Stream => ({
  id,
  source_url: `rtsp://camera-${index}.local:554/stream`,
  status: ['active', 'inactive', 'error'][index % 3] as any,
  created_at: new Date(Date.now() - index * 86400000).toISOString(),
  recording: {
    enabled: index % 2 === 0,
    status: index % 2 === 0 ? 'recording' : 'stopped',
    current_file: index % 2 === 0 ? `/recordings/${id}/current.mp4` : undefined,
    duration: index % 2 === 0 ? 3600 + index * 100 : undefined
  },
  metrics: {
    bitrate: 4096000 + index * 100000,
    framerate: 30,
    resolution: '1920x1080',
    packets_received: 1000000 + index * 10000,
    packets_lost: index * 10
  },
  pipeline: {
    state: 'playing',
    latency: 150 + index * 10,
    buffer_level: 80 - index * 5
  }
});

// Mock API implementation
export class MockAPIClient {
  private streams: Map<string, Stream> = new Map();
  private config: SystemConfig;

  constructor() {
    // Initialize with some mock data
    for (let i = 1; i <= 5; i++) {
      const id = `camera-${i}`;
      this.streams.set(id, generateMockStream(id, i));
    }

    this.config = {
      server: {
        port: 3000,
        host: '0.0.0.0'
      },
      recording: {
        base_path: '/recordings',
        segment_duration: 600,
        retention_days: 7
      },
      inference: {
        enabled: true,
        device: 'gpu',
        models_path: '/models'
      }
    };
  }

  async delay(ms = 100): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  // Health API
  async checkHealth(): Promise<HealthResponse> {
    await this.delay();
    return {
      status: 'healthy',
      uptime: 3600,
      version: '0.1.0'
    };
  }

  async getStatus(): Promise<SystemStatus> {
    await this.delay();
    const activeStreams = Array.from(this.streams.values())
      .filter(s => s.status === 'active').length;
    const recordingStreams = Array.from(this.streams.values())
      .filter(s => s.recording?.status === 'recording').length;

    return {
      active_streams: activeStreams,
      total_streams: this.streams.size,
      recording_streams: recordingStreams,
      cpu_usage: 45.2 + Math.random() * 10,
      memory_usage: 2048 + Math.floor(Math.random() * 512),
      disk_usage: {
        used: 104857600,
        total: 1073741824
      }
    };
  }

  // Streams API
  async listStreams(): Promise<StreamListResponse> {
    await this.delay();
    return {
      streams: Array.from(this.streams.values())
    };
  }

  async getStream(id: string): Promise<Stream> {
    await this.delay();
    const stream = this.streams.get(id);
    if (!stream) {
      throw new Error(`Stream ${id} not found`);
    }
    return stream;
  }

  async createStream(data: any): Promise<Stream> {
    await this.delay(200);
    const stream = generateMockStream(data.id, this.streams.size + 1);
    this.streams.set(data.id, stream);
    return stream;
  }

  async updateStream(id: string, data: any): Promise<void> {
    await this.delay();
    const stream = this.streams.get(id);
    if (!stream) {
      throw new Error(`Stream ${id} not found`);
    }
    Object.assign(stream, data);
  }

  async deleteStream(id: string): Promise<void> {
    await this.delay();
    if (!this.streams.has(id)) {
      throw new Error(`Stream ${id} not found`);
    }
    this.streams.delete(id);
  }

  async startStream(id: string): Promise<{ status: string }> {
    await this.delay(500);
    const stream = this.streams.get(id);
    if (!stream) {
      throw new Error(`Stream ${id} not found`);
    }
    stream.status = 'active';
    return { status: 'starting' };
  }

  async stopStream(id: string): Promise<{ status: string }> {
    await this.delay(500);
    const stream = this.streams.get(id);
    if (!stream) {
      throw new Error(`Stream ${id} not found`);
    }
    stream.status = 'inactive';
    return { status: 'stopping' };
  }

  // Recording API
  async startRecording(id: string): Promise<any> {
    await this.delay();
    const stream = this.streams.get(id);
    if (!stream) {
      throw new Error(`Stream ${id} not found`);
    }
    if (stream.recording) {
      stream.recording.status = 'recording';
      stream.recording.current_file = `/recordings/${id}/new.mp4`;
    }
    return {
      status: 'recording',
      filename: `/recordings/${id}/new.mp4`
    };
  }

  async stopRecording(id: string): Promise<any> {
    await this.delay();
    const stream = this.streams.get(id);
    if (!stream) {
      throw new Error(`Stream ${id} not found`);
    }
    if (stream.recording) {
      stream.recording.status = 'stopped';
      stream.recording.current_file = undefined;
    }
    return {
      status: 'stopped',
      files: [`/recordings/${id}/file1.mp4`],
      total_duration: 1200,
      total_size: 209715200
    };
  }

  async listRecordings(id: string): Promise<RecordingListResponse> {
    await this.delay();
    return {
      recordings: [
        {
          filename: 'recording1.mp4',
          path: `/recordings/${id}/recording1.mp4`,
          size: 104857600,
          duration: 600,
          created_at: new Date().toISOString()
        }
      ],
      total_size: 104857600,
      total_duration: 600
    };
  }

  // Metrics API
  async getStreamMetrics(id: string): Promise<DetailedMetrics> {
    await this.delay();
    return {
      bitrate: {
        current: 4096000,
        average: 4000000,
        peak: 5000000
      },
      framerate: {
        current: 30,
        average: 29.97,
        dropped: 5
      },
      latency: {
        pipeline: 150,
        network: 50,
        processing: 100
      },
      packets: {
        received: 1000000,
        lost: 10,
        recovered: 8
      },
      errors: {
        decode: 0,
        network: 2,
        total: 2
      }
    };
  }

  // Config API
  async getConfig(): Promise<SystemConfig> {
    await this.delay();
    return this.config;
  }

  async updateConfig(data: any): Promise<void> {
    await this.delay();
    Object.assign(this.config, data);
  }

  async reloadConfig(): Promise<void> {
    await this.delay(500);
  }

}

// Create mock API that mimics the real API interface
export function createMockAPI() {
  const mockClient = new MockAPIClient();

  return {
    client: mockClient,
    auth: {
      login: async (credentials: { username: string; password: string }) => {
        await mockClient.delay();
        if (credentials.username === 'dev' && credentials.password === 'dev') {
          return {
            token: 'mock-token-' + Date.now(),
            user: {
              id: 'dev-user',
              username: 'developer',
              email: 'dev@example.com',
              role: 'admin'
            },
            expiresIn: 3600
          };
        }
        throw new Error('Invalid credentials');
      },
      logout: async () => {
        await mockClient.delay();
      },
      verify: async () => {
        await mockClient.delay();
        return {
          id: 'dev-user',
          username: 'developer',
          email: 'dev@example.com',
          role: 'admin'
        };
      },
      refresh: async () => {
        await mockClient.delay();
        return {
          token: 'mock-token-' + Date.now(),
          expiresIn: 3600
        };
      }
    },
    health: {
      check: () => mockClient.checkHealth(),
      getStatus: () => mockClient.getStatus(),
      isHealthy: async () => {
        try {
          const health = await mockClient.checkHealth();
          return health.status === 'healthy';
        } catch {
          return false;
        }
      }
    },
    streams: {
      list: () => mockClient.listStreams(),
      get: (id: string) => mockClient.getStream(id),
      create: (data: any) => mockClient.createStream(data),
      update: (id: string, data: any) => mockClient.updateStream(id, data),
      delete: (id: string) => mockClient.deleteStream(id),
      start: (id: string) => mockClient.startStream(id),
      stop: (id: string) => mockClient.stopStream(id),
      restart: async (id: string) => {
        await mockClient.stopStream(id);
        await mockClient.startStream(id);
        return { status: 'restarting' };
      },
      startRecording: (id: string) => mockClient.startRecording(id),
      stopRecording: (id: string) => mockClient.stopRecording(id),
      listRecordings: (id: string) => mockClient.listRecordings(id),
      getMetrics: (id: string) => mockClient.getStreamMetrics(id)
    },
    config: {
      get: () => mockClient.getConfig(),
      update: (data: any) => mockClient.updateConfig(data),
      reload: () => mockClient.reloadConfig()
    },
    setToken: (token: string | null) => { /* Mock */ },
    clearCache: (pattern?: string) => { /* Mock */ },
    cancelAll: () => { /* Mock */ }
  };
}