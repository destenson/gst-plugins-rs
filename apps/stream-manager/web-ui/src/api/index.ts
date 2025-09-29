import { APIClient } from './client.ts';
import { StreamsAPI } from './services/StreamsAPI.ts';
import { ConfigAPI } from './services/ConfigAPI.ts';
import { MetricsAPI } from './services/MetricsAPI.ts';
import { HealthAPI } from './services/HealthAPI.ts';

// Export types
export * from './types/index.ts';
export { APIClient, type APIClientConfig } from './client.ts';

// Main API class that combines all services
export class StreamManagerAPI {
  public client: APIClient;
  public streams: StreamsAPI;
  public config: ConfigAPI;
  public metrics: MetricsAPI;
  public health: HealthAPI;

  constructor(config?: { baseURL?: string; token?: string }) {
    this.client = new APIClient(config);

    // Initialize all API services
    this.streams = new StreamsAPI(this.client);
    this.config = new ConfigAPI(this.client);
    this.metrics = new MetricsAPI(this.client);
    this.health = new HealthAPI(this.client);
  }

  setToken(token: string | null): void {
    this.client.setToken(token);
  }

  clearCache(pattern?: string): void {
    this.client.clearCache(pattern);
  }

  cancelAll(): void {
    this.client.cancelAll();
  }
}

// Create default instance
const api = new StreamManagerAPI();

// Export for browser window access (for debugging)
if (typeof window !== 'undefined') {
  (window as any).api = api;
}

export default api;