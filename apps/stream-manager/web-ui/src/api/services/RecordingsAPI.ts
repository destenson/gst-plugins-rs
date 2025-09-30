import { APIClient } from "../client.ts";
import type { Recording, RecordingListQuery, RecordingListResponse } from "../types/index.ts";

export class RecordingsAPI {
  constructor(private client: APIClient) {}

  async list(query?: RecordingListQuery & { stream_id?: string }): Promise<RecordingListResponse> {
    return this.client.get<RecordingListResponse>("/api/v1/recordings", {
      params: query,
      cancelKey: "recordings-list",
    });
  }

  async get(filename: string): Promise<Recording> {
    return this.client.get<Recording>(`/api/v1/recordings/${filename}`, {
      cancelKey: `recording-${filename}`,
    });
  }

  async delete(filename: string): Promise<void> {
    return this.client.delete<void>(`/api/v1/recordings/${filename}`);
  }

  async download(filename: string): Promise<Blob> {
    // For file downloads, we need to handle the response differently
    const response = await this.client.get<Blob>(`/api/v1/recordings/${filename}/download`, {
      responseType: "blob",
    });
    return response;
  }
}
