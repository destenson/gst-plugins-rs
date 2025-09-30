import axios, { AxiosInstance, AxiosRequestConfig, AxiosError, CancelTokenSource } from 'axios';
import axiosRetry from 'axios-retry';
import type { APIError } from './types/index.ts';

// Configuration
export interface APIClientConfig {
  baseURL?: string;
  token?: string | null;
  timeout?: number;
  retryAttempts?: number;
  retryDelay?: number;
  cacheEnabled?: boolean;
  cacheMaxAge?: number;
}

// Cache implementation
interface CacheEntry {
  data: any;
  timestamp: number;
  maxAge: number;
}

class ResponseCache {
  private cache: Map<string, CacheEntry> = new Map();

  get(key: string): any | null {
    const entry = this.cache.get(key);
    if (!entry) return null;

    const age = Date.now() - entry.timestamp;
    if (age > entry.maxAge) {
      this.cache.delete(key);
      return null;
    }

    return entry.data;
  }

  set(key: string, data: any, maxAge: number): void {
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
      maxAge
    });
  }

  clear(): void {
    this.cache.clear();
  }

  delete(pattern?: string): void {
    if (!pattern) {
      this.clear();
      return;
    }

    Array.from(this.cache.keys()).forEach(key => {
      if (key.includes(pattern)) {
        this.cache.delete(key);
      }
    });
  }
}

// Main API Client class
export class APIClient {
  private axios: AxiosInstance;
  private cache: ResponseCache;
  private config: APIClientConfig;
  private cancelTokenSources: Map<string, CancelTokenSource> = new Map();

  constructor(config: APIClientConfig = {}) {
    this.config = {
      baseURL: config.baseURL || '',
      timeout: config.timeout || 30000,
      retryAttempts: config.retryAttempts || 1, // Reduce from 3 to 1
      retryDelay: config.retryDelay || 2000, // Increase delay
      cacheEnabled: config.cacheEnabled ?? true,
      cacheMaxAge: config.cacheMaxAge || 60000, // 1 minute default
      ...config
    };

    this.cache = new ResponseCache();

    // Create axios instance
    this.axios = axios.create({
      baseURL: this.config.baseURL,
      timeout: this.config.timeout,
      headers: {
        'Content-Type': 'application/json'
      }
    });

    this.setupInterceptors();
    this.setupRetry();
  }

  private setupInterceptors(): void {
    // Request interceptor for authentication
    this.axios.interceptors.request.use(
      (config) => {
        // Add auth token if available
        const token = this.getToken();
        if (token) {
          config.headers = config.headers || {};
          config.headers['Authorization'] = `Bearer ${token}`;
        }

        // Add cache check for GET requests
        if (this.config.cacheEnabled && config.method === 'get') {
          const cacheKey = this.getCacheKey(config);
          const cachedData = this.cache.get(cacheKey);
          if (cachedData) {
            // Return cached data by rejecting with special flag
            return Promise.reject({
              __cached: true,
              data: cachedData
            });
          }
        }

        return config;
      },
      (error) => {
        return Promise.reject(error);
      }
    );

    // Response interceptor for error handling and caching
    this.axios.interceptors.response.use(
      (response) => {
        // Cache successful GET responses
        if (this.config.cacheEnabled && response.config.method === 'get') {
          const cacheKey = this.getCacheKey(response.config);
          this.cache.set(cacheKey, response.data, this.config.cacheMaxAge!);
        }

        return response;
      },
      (error: AxiosError<APIError>) => {
        // Handle cached response
        if (error && (error as any).__cached) {
          return Promise.resolve({
            data: (error as any).data,
            status: 200,
            statusText: 'OK (Cached)',
            headers: {},
            config: {} as any
          });
        }

        // Handle authentication errors
        if (error.response?.status === 401) {
          this.handleAuthError();
        }

        // Format error for consistent handling
        if (error.response?.data?.error) {
          const apiError = error.response.data.error;
          return Promise.reject({
            code: apiError.code,
            message: apiError.message,
            details: apiError.details,
            status: error.response.status
          });
        }

        // Network or timeout errors
        if (!error.response) {
          return Promise.reject({
            code: 'NETWORK_ERROR',
            message: error.message || 'Network error occurred',
            details: { originalError: error }
          });
        }

        return Promise.reject(error);
      }
    );
  }

  private setupRetry(): void {
    axiosRetry(this.axios, {
      retries: this.config.retryAttempts!,
      retryDelay: (retryCount) => {
        return retryCount * this.config.retryDelay!;
      },
      retryCondition: (error) => {
        // Retry on network errors or 5xx errors
        return axiosRetry.isNetworkOrIdempotentRequestError(error) ||
               (error.response?.status ?? 0) >= 500;
      },
      onRetry: (retryCount, error, requestConfig) => {
        console.log(`Retry attempt ${retryCount} for ${requestConfig.url}`);
      }
    });
  }

  private getCacheKey(config: AxiosRequestConfig): string {
    const params = config.params ? JSON.stringify(config.params) : '';
    return `${config.method}:${config.url}:${params}`;
  }

  private getToken(): string | null {
    // First check if token was provided in config
    if (this.config.token) {
      return this.config.token;
    }

    // Then check localStorage
    const storedToken = localStorage.getItem('api_token');
    if (storedToken) {
      return storedToken;
    }

    return null;
  }

  private handleAuthError(): void {
    // Clear token and redirect to login
    localStorage.removeItem('api_token');
    this.config.token = null;

    // Dispatch custom event for auth failure
    globalThis.dispatchEvent(new CustomEvent('auth:failed'));
  }

  // Public methods

  setToken(token: string | null): void {
    this.config.token = token;
    if (token) {
      localStorage.setItem('api_token', token);
    } else {
      localStorage.removeItem('api_token');
    }
  }

  clearCache(pattern?: string): void {
    this.cache.delete(pattern);
  }

  // Request methods with cancellation support

  async get<T>(url: string, config?: AxiosRequestConfig & { cancelKey?: string }): Promise<T> {
    this.setupCancellation(config?.cancelKey);
    const response = await this.axios.get<T>(url, this.getCancelConfig(config));
    return response.data;
  }

  async post<T>(url: string, data?: any, config?: AxiosRequestConfig & { cancelKey?: string }): Promise<T> {
    this.setupCancellation(config?.cancelKey);
    this.clearCache(url); // Clear cache for this endpoint
    const response = await this.axios.post<T>(url, data, this.getCancelConfig(config));
    return response.data;
  }

  async put<T>(url: string, data?: any, config?: AxiosRequestConfig & { cancelKey?: string }): Promise<T> {
    this.setupCancellation(config?.cancelKey);
    this.clearCache(url); // Clear cache for this endpoint
    const response = await this.axios.put<T>(url, data, this.getCancelConfig(config));
    return response.data;
  }

  async delete<T>(url: string, config?: AxiosRequestConfig & { cancelKey?: string }): Promise<T> {
    this.setupCancellation(config?.cancelKey);
    this.clearCache(url); // Clear cache for this endpoint
    const response = await this.axios.delete<T>(url, this.getCancelConfig(config));
    return response.data;
  }

  async patch<T>(url: string, data?: any, config?: AxiosRequestConfig & { cancelKey?: string }): Promise<T> {
    this.setupCancellation(config?.cancelKey);
    this.clearCache(url); // Clear cache for this endpoint
    const response = await this.axios.patch<T>(url, data, this.getCancelConfig(config));
    return response.data;
  }

  // Cancellation handling

  private setupCancellation(key?: string): void {
    if (!key) return;

    // Cancel existing request with same key
    this.cancel(key);

    // Create new cancel token
    const source = axios.CancelToken.source();
    this.cancelTokenSources.set(key, source);
  }

  private getCancelConfig(config?: AxiosRequestConfig & { cancelKey?: string }): AxiosRequestConfig {
    if (!config?.cancelKey) return config || {};

    const source = this.cancelTokenSources.get(config.cancelKey);
    if (!source) return config;

    const { cancelKey, ...axiosConfig } = config;
    return {
      ...axiosConfig,
      cancelToken: source.token
    };
  }

  cancel(key: string): void {
    const source = this.cancelTokenSources.get(key);
    if (source) {
      source.cancel(`Request cancelled: ${key}`);
      this.cancelTokenSources.delete(key);
    }
  }

  cancelAll(): void {
    this.cancelTokenSources.forEach((source, key) => {
      source.cancel(`Request cancelled: ${key}`);
    });
    this.cancelTokenSources.clear();
  }

  // Utility method to check if error is cancellation
  static isCancel(error: any): boolean {
    return axios.isCancel(error);
  }
}

// Default instance
export const apiClient = new APIClient();

export default apiClient;