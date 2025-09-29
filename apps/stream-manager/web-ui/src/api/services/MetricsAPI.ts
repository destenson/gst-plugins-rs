import { APIClient } from '../client.ts';

export class MetricsAPI {
  constructor(private client: APIClient) {}

  async getPrometheus(): Promise<string> {
    const response = await this.client.get<any>('/api/v1/metrics', {
      cancelKey: 'metrics-prometheus',
      headers: {
        'Accept': 'text/plain'
      }
    });
    return response as string;
  }

  async parseMetrics(): Promise<Map<string, number>> {
    const raw = await this.getPrometheus();
    const metrics = new Map<string, number>();

    const lines = raw.split('\n');
    for (const line of lines) {
      if (line.startsWith('#') || !line.trim()) continue;

      const match = line.match(/^([a-zA-Z_][a-zA-Z0-9_]*(?:\{[^}]+\})?)\s+(\d+(?:\.\d+)?)/);
      if (match) {
        metrics.set(match[1], parseFloat(match[2]));
      }
    }

    return metrics;
  }

  async getActiveStreams(): Promise<number> {
    const metrics = await this.parseMetrics();
    return metrics.get('stream_manager_active_streams') || 0;
  }

  async getBytesReceived(streamId?: string): Promise<number> {
    const metrics = await this.parseMetrics();
    const key = streamId
      ? `stream_manager_bytes_received{stream="${streamId}"}`
      : 'stream_manager_bytes_received';
    return metrics.get(key) || 0;
  }

  async getPacketsLost(streamId?: string): Promise<number> {
    const metrics = await this.parseMetrics();
    const key = streamId
      ? `stream_manager_packets_lost{stream="${streamId}"}`
      : 'stream_manager_packets_lost';
    return metrics.get(key) || 0;
  }
}
