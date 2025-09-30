import { APIClient } from "../client.ts";

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  token: string;
  user: {
    id: string;
    username: string;
    email?: string;
    role?: string;
  };
  expiresIn?: number;
}

export interface VerifyResponse {
  id: string;
  username: string;
  email?: string;
  role?: string;
}

export class AuthAPI {
  constructor(private client: APIClient) {}

  /**
   * Login with username and password
   */
  async login(credentials: LoginRequest): Promise<LoginResponse> {
    return this.client.post<LoginResponse>("/auth/login", credentials);
  }

  /**
   * Logout and invalidate token
   */
  async logout(): Promise<void> {
    return this.client.post<void>("/auth/logout", {});
  }

  /**
   * Verify current token and get user info
   */
  async verify(): Promise<VerifyResponse> {
    return this.client.get<VerifyResponse>("/auth/verify");
  }

  /**
   * Refresh authentication token
   */
  async refresh(): Promise<{ token: string; expiresIn?: number }> {
    return this.client.post<{ token: string; expiresIn?: number }>("/auth/refresh", {});
  }
}
