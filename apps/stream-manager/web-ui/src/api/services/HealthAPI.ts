import { APIClient } from '../client.ts';
import type {
  HealthResponse,
  SystemStatus
} from '../types/index.ts';

export class HealthAPI {
  constructor(private client: APIClient) {}

  async check(): Promise<HealthResponse> {
    return this.client.get<HealthResponse>('/api/v1/health', {
      cancelKey: 'health-check'
    });
  }

  async getStatus(): Promise<SystemStatus> {
    return this.client.get<SystemStatus>('/api/v1/metrics', {
      cancelKey: 'system-status'
    });
  }

  async isHealthy(): Promise<boolean> {
    try {
      const health = await this.check();
      return health.status === 'healthy';
    } catch {
      return false;
    }
  }

  async waitForHealthy(maxAttempts = 30, interval = 1000): Promise<boolean> {
    for (let i = 0; i < maxAttempts; i++) {
      if (await this.isHealthy()) {
        return true;
      }
      await new Promise(resolve => setTimeout(resolve, interval));
    }
    return false;
  }
}