import { APIClient } from '../client.ts';
import type {
  Stream,
  StreamListResponse,
  CreateStreamRequest,
  UpdateStreamRequest,
  StreamListQuery,
  StartRecordingRequest,
  StartRecordingResponse,
  StopRecordingResponse,
  RecordingListResponse,
  RecordingListQuery,
  DetailedMetrics
} from '../types/index.ts';

export class StreamsAPI {
  constructor(private client: APIClient) {}

  // Stream Management

  async list(query?: StreamListQuery): Promise<StreamListResponse> {
    return this.client.get<StreamListResponse>('/api/streams', {
      params: query,
      cancelKey: 'streams-list'
    });
  }

  async get(id: string): Promise<Stream> {
    return this.client.get<Stream>(`/api/streams/${id}`, {
      cancelKey: `stream-${id}`
    });
  }

  async create(data: CreateStreamRequest): Promise<Stream> {
    return this.client.post<Stream>('/api/streams', data);
  }

  async update(id: string, data: UpdateStreamRequest): Promise<void> {
    return this.client.put<void>(`/api/streams/${id}`, data);
  }

  async delete(id: string): Promise<void> {
    return this.client.delete<void>(`/api/streams/${id}`);
  }

  // Stream Control

  async start(id: string): Promise<{ status: string }> {
    return this.client.post<{ status: string }>(`/api/streams/${id}/start`);
  }

  async stop(id: string): Promise<{ status: string }> {
    return this.client.post<{ status: string }>(`/api/streams/${id}/stop`);
  }

  async restart(id: string): Promise<{ status: string }> {
    return this.client.post<{ status: string }>(`/api/streams/${id}/restart`);
  }

  // Recording Control

  async startRecording(id: string, data?: StartRecordingRequest): Promise<StartRecordingResponse> {
    return this.client.post<StartRecordingResponse>(
      `/api/streams/${id}/recording/start`,
      data
    );
  }

  async stopRecording(id: string): Promise<StopRecordingResponse> {
    return this.client.post<StopRecordingResponse>(`/api/streams/${id}/recording/stop`);
  }

  async pauseRecording(id: string): Promise<void> {
    return this.client.post<void>(`/api/streams/${id}/recording/pause`);
  }

  async resumeRecording(id: string): Promise<void> {
    return this.client.post<void>(`/api/streams/${id}/recording/resume`);
  }

  async listRecordings(id: string, query?: RecordingListQuery): Promise<RecordingListResponse> {
    return this.client.get<RecordingListResponse>(`/api/streams/${id}/recordings`, {
      params: query,
      cancelKey: `recordings-${id}`
    });
  }

  // Metrics

  async getMetrics(id: string): Promise<DetailedMetrics> {
    return this.client.get<DetailedMetrics>(`/api/streams/${id}/metrics`, {
      cancelKey: `metrics-${id}`
    });
  }

  // Batch operations

  async startAll(): Promise<void> {
    const { streams } = await this.list();
    await Promise.all(
      streams
        .filter((s: Stream) => s.status === 'inactive')
        .map((s: Stream) => this.start(s.id))
    );
  }

  async stopAll(): Promise<void> {
    const { streams } = await this.list();
    await Promise.all(
      streams
        .filter((s: Stream) => s.status === 'active')
        .map((s: Stream) => this.stop(s.id))
    );
  }
}