import { APIClient } from "../client.ts";
import type { SystemConfig, UpdateConfigRequest } from "../types/index.ts";

export class ConfigAPI {
  constructor(private client: APIClient) {}

  async get(): Promise<SystemConfig> {
    return this.client.get<SystemConfig>("/api/v1/config", {
      cancelKey: "config-get",
    });
  }

  async update(data: UpdateConfigRequest): Promise<void> {
    return this.client.put<void>("/api/v1/config", data);
  }

  async reload(): Promise<void> {
    return this.client.post<void>("/api/v1/config/reload");
  }

  // Utility methods

  async getRecordingPath(): Promise<string> {
    const config = await this.get();
    return config.recording.base_path;
  }

  async updateRetentionDays(days: number): Promise<void> {
    return this.update({
      recording: { retention_days: days },
    });
  }

  async updateSegmentDuration(seconds: number): Promise<void> {
    return this.update({
      recording: { segment_duration: seconds },
    });
  }

  async toggleInference(enabled: boolean): Promise<void> {
    return this.update({
      inference: { enabled },
    });
  }
}
