import { useCallback, useEffect, useRef, useState } from "react";
import { useWebSocketContext } from "./context.tsx";

// Re-export for convenience
export { useWebSocketContext };
import { ConnectionState, EventType, SubscriptionRequest, WebSocketEvent } from "./types.ts";

// Hook to access the WebSocket client and connection state
export function useWebSocket() {
  const context = useWebSocketContext();
  return {
    connectionState: context.connectionState,
    isConnected: context.isConnected,
    clientId: context.clientId,
    connect: context.connect,
    disconnect: context.disconnect,
    subscribe: context.subscribe,
    send: context.send,
    sendJSON: context.sendJSON,
  };
}

// Hook to listen for specific WebSocket events
export function useWebSocketEvent<T = any>(
  eventType: EventType | EventType[],
  handler: (event: WebSocketEvent<T>) => void,
  deps: React.DependencyList = [],
) {
  const { client } = useWebSocketContext();
  const handlerRef = useRef(handler);

  // Update handler ref when it changes
  useEffect(() => {
    handlerRef.current = handler;
  }, [handler]);

  useEffect(() => {
    if (!client) return;

    const eventTypes = Array.isArray(eventType) ? eventType : [eventType];

    const messageHandler = (event: WebSocketEvent) => {
      if (eventTypes.includes(event.event_type)) {
        handlerRef.current(event as WebSocketEvent<T>);
      }
    };

    client.on("message", messageHandler);

    return () => {
      client.removeListener("message", messageHandler);
    };
  }, [client, eventType, ...deps]);
}

// Hook to subscribe to WebSocket events with automatic subscription management
export function useWebSocketSubscription(
  request: SubscriptionRequest,
  onEvent?: (event: WebSocketEvent) => void,
) {
  const { client, subscribe, isConnected } = useWebSocketContext();
  const [events, setEvents] = useState<WebSocketEvent[]>([]);
  const subscriptionSent = useRef(false);

  // Send subscription when connected
  useEffect(() => {
    if (isConnected && !subscriptionSent.current) {
      subscribe(request);
      subscriptionSent.current = true;
    } else if (!isConnected) {
      subscriptionSent.current = false;
    }
  }, [isConnected, subscribe, request]);

  // Listen for events
  useEffect(() => {
    if (!client) return;

    const handleMessage = (event: WebSocketEvent) => {
      // Check if event matches subscription
      const matchesEventType = !request.event_types ||
        request.event_types.includes(event.event_type);
      const matchesStreamId = !request.stream_ids ||
        (event.stream_id && request.stream_ids.includes(event.stream_id));

      if (matchesEventType && matchesStreamId) {
        setEvents((prev) => [...prev, event]);
        onEvent?.(event);
      }
    };

    client.on("message", handleMessage);

    return () => {
      client.removeListener("message", handleMessage);
    };
  }, [client, request, onEvent]);

  const clearEvents = useCallback(() => {
    setEvents([]);
  }, []);

  return { events, clearEvents };
}

// Hook to track connection state changes
export function useConnectionState(
  onStateChange?: (state: ConnectionState) => void,
) {
  const { connectionState, client } = useWebSocketContext();
  const [stateHistory, setStateHistory] = useState<
    Array<{ state: ConnectionState; timestamp: Date }>
  >([]);

  useEffect(() => {
    if (!client) return;

    const handleStateChange = (state: ConnectionState) => {
      setStateHistory((prev) => [...prev, { state, timestamp: new Date() }]);
      onStateChange?.(state);
    };

    client.on("connectionStateChange", handleStateChange);

    return () => {
      client.removeListener("connectionStateChange", handleStateChange);
    };
  }, [client, onStateChange]);

  return {
    currentState: connectionState,
    stateHistory,
    isConnected: connectionState === ConnectionState.Connected,
    isConnecting: connectionState === ConnectionState.Connecting,
    isReconnecting: connectionState === ConnectionState.Reconnecting,
    hasError: connectionState === ConnectionState.Error,
  };
}

// Hook for stream-specific events
export function useStreamEvents(streamId: string) {
  const [lastHealth, setLastHealth] = useState<string | null>(null);
  const [isRecording, setIsRecording] = useState(false);
  const [statistics, setStatistics] = useState<any>(null);

  // Listen for health changes
  useWebSocketEvent(
    EventType.StreamHealthChanged,
    (event) => {
      if (event.stream_id === streamId) {
        setLastHealth(event.data.health);
      }
    },
    [streamId],
  );

  // Listen for recording events
  useWebSocketEvent(
    [EventType.RecordingStarted, EventType.RecordingStopped],
    (event) => {
      if (event.stream_id === streamId) {
        setIsRecording(event.event_type === EventType.RecordingStarted);
      }
    },
    [streamId],
  );

  // Listen for statistics
  useWebSocketEvent(
    EventType.StatisticsUpdate,
    (event) => {
      if (event.data.streams && event.data.streams[streamId]) {
        setStatistics(event.data.streams[streamId]);
      }
    },
    [streamId],
  );

  return {
    health: lastHealth,
    isRecording,
    statistics,
  };
}

// Hook for system-wide events
export function useSystemEvents() {
  const [alerts, setAlerts] = useState<Array<{ message: string; level?: string; timestamp: Date }>>(
    [],
  );
  const [errors, setErrors] = useState<Array<{ error: string; details?: string; timestamp: Date }>>(
    [],
  );
  const [systemStats, setSystemStats] = useState<any>(null);

  // Listen for system alerts
  useWebSocketEvent(
    EventType.SystemAlert,
    (event) => {
      setAlerts((prev) => [...prev, {
        message: event.data.message,
        level: event.data.level,
        timestamp: new Date(event.timestamp),
      }]);
    },
  );

  // Listen for errors
  useWebSocketEvent(
    EventType.ErrorOccurred,
    (event) => {
      setErrors((prev) => [...prev, {
        error: event.data.error,
        details: event.data.details,
        timestamp: new Date(event.timestamp),
      }]);
    },
  );

  // Listen for system statistics
  useWebSocketEvent(
    EventType.StatisticsUpdate,
    (event) => {
      if (event.data.system) {
        setSystemStats(event.data.system);
      }
    },
  );

  const clearAlerts = useCallback(() => setAlerts([]), []);
  const clearErrors = useCallback(() => setErrors([]), []);

  return {
    alerts,
    errors,
    systemStats,
    clearAlerts,
    clearErrors,
  };
}
