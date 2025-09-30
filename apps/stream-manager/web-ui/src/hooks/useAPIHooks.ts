import { useCallback, useEffect, useRef, useState } from "react";
import { useAPI } from "../contexts/APIContext.tsx";
import { APIClient } from "../api/client.ts";
import type {
  DetailedMetrics,
  Stream,
  StreamListResponse,
  SystemConfig,
  SystemStatus,
} from "../api/types/index.ts";

// Generic hook for API calls with loading and error states
interface UseAPICallResult<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
  cancel: () => void;
}

function useAPICall<T>(
  apiCall: () => Promise<T>,
  deps: any[] = [],
  autoFetch = true,
): UseAPICallResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(autoFetch);
  const [error, setError] = useState<Error | null>(null);
  const cancelKey = useRef<string>("");

  const fetch = useCallback(async () => {
    setLoading(true);
    setError(null);
    cancelKey.current = `api-call-${Date.now()}`;

    try {
      const result = await apiCall();
      setData(result);
    } catch (err) {
      if (!APIClient.isCancel(err)) {
        setError(err as Error);
      }
    } finally {
      setLoading(false);
    }
  }, deps);

  const cancel = useCallback(() => {
    const { api } = useAPI();
    if (cancelKey.current && api.cancelAll) {
      api.cancelAll();
    }
  }, []);

  useEffect(() => {
    if (autoFetch) {
      fetch();
    }

    return () => {
      cancel();
    };
  }, deps);

  return { data, loading, error, refetch: fetch, cancel };
}

// Hook for fetching all streams
export function useStreams(autoFetch = true) {
  const { api } = useAPI();
  return useAPICall<StreamListResponse>(
    () => api.streams.list(),
    [api],
    autoFetch,
  );
}

// Hook for fetching a single stream
export function useStream(id: string | null, autoFetch = true) {
  const { api } = useAPI();
  return useAPICall<Stream | null>(
    async () => {
      if (!id) return null;
      return api.streams.get(id);
    },
    [api, id],
    autoFetch && !!id,
  );
}

// Hook for system status
export function useSystemStatus(refreshInterval?: number) {
  const { api } = useAPI();
  const result = useAPICall<SystemStatus>(
    () => api.health.getStatus(),
    [api],
    true,
  );

  useEffect(() => {
    if (refreshInterval && refreshInterval > 0) {
      const interval = setInterval(() => {
        result.refetch();
      }, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [refreshInterval]);

  return result;
}

// Hook for stream metrics
export function useStreamMetrics(id: string | null, refreshInterval?: number) {
  const { api } = useAPI();
  const result = useAPICall<DetailedMetrics | null>(
    async () => {
      if (!id) return null;
      return api.streams.getMetrics(id);
    },
    [api, id],
    !!id,
  );

  useEffect(() => {
    if (refreshInterval && refreshInterval > 0 && id) {
      const interval = setInterval(() => {
        result.refetch();
      }, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [refreshInterval, id]);

  return result;
}

// Hook for system configuration
export function useSystemConfig() {
  const { api } = useAPI();
  return useAPICall<SystemConfig>(
    () => api.config.get(),
    [api],
    true,
  );
}

// Hook for stream actions with optimistic updates
export function useStreamActions() {
  const { api } = useAPI();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const execute = useCallback(async (
    action: () => Promise<any>,
    onSuccess?: () => void,
    onError?: (error: Error) => void,
  ) => {
    setLoading(true);
    setError(null);

    try {
      await action();
      onSuccess?.();
    } catch (err) {
      const error = err as Error;
      setError(error);
      onError?.(error);
    } finally {
      setLoading(false);
    }
  }, []);

  const startStream = useCallback((id: string, onSuccess?: () => void) => {
    return execute(() => api.streams.start(id), onSuccess);
  }, [api, execute]);

  const stopStream = useCallback((id: string, onSuccess?: () => void) => {
    return execute(() => api.streams.stop(id), onSuccess);
  }, [api, execute]);

  const restartStream = useCallback((id: string, onSuccess?: () => void) => {
    return execute(() => api.streams.restart(id), onSuccess);
  }, [api, execute]);

  const deleteStream = useCallback((id: string, onSuccess?: () => void) => {
    return execute(() => api.streams.delete(id), onSuccess);
  }, [api, execute]);

  const startRecording = useCallback((id: string, onSuccess?: () => void) => {
    return execute(() => api.streams.startRecording(id), onSuccess);
  }, [api, execute]);

  const stopRecording = useCallback((id: string, onSuccess?: () => void) => {
    return execute(() => api.streams.stopRecording(id), onSuccess);
  }, [api, execute]);

  return {
    loading,
    error,
    startStream,
    stopStream,
    restartStream,
    deleteStream,
    startRecording,
    stopRecording,
  };
}

// Hook for health check with auto-retry
export function useHealthCheck(interval = 5000) {
  const { api } = useAPI();
  const [isHealthy, setIsHealthy] = useState<boolean | null>(null);
  const [lastCheck, setLastCheck] = useState<Date | null>(null);

  useEffect(() => {
    const checkHealth = async () => {
      try {
        const health = await api.health.check();
        setIsHealthy(health.status === "healthy");
        setLastCheck(new Date());
      } catch {
        setIsHealthy(false);
        setLastCheck(new Date());
      }
    };

    checkHealth();
    const intervalId = setInterval(checkHealth, interval);
    return () => clearInterval(intervalId);
  }, [api, interval]);

  return { isHealthy, lastCheck };
}

// Hook for WebSocket events (placeholder for WebSocket implementation)
export function useWebSocketEvents(eventTypes?: string[]) {
  const [events, setEvents] = useState<any[]>([]);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    // TODO: Implement WebSocket connection
    // This is a placeholder for the WebSocket implementation
    // which should be done in a separate PRP
    console.log("WebSocket events hook - not yet implemented");
  }, [eventTypes]);

  return { events, connected };
}
