import React, { createContext, useCallback, useContext, useEffect, useRef, useState } from "react";
import { WebSocketClient } from "./client.ts";
import {
  ConnectionState,
  EventType,
  SubscriptionRequest,
  WebSocketConfig,
  WebSocketEvent,
} from "./types.ts";

interface WebSocketContextValue {
  client: WebSocketClient | null;
  connectionState: ConnectionState;
  lastEvent: WebSocketEvent | null;
  connect: () => void;
  disconnect: () => void;
  subscribe: (request: SubscriptionRequest) => void;
  send: (message: string | ArrayBuffer) => void;
  sendJSON: (data: any) => void;
  isConnected: boolean;
  clientId: string | null;
}

const WebSocketContext = createContext<WebSocketContextValue | null>(null);

export interface WebSocketProviderProps {
  children: React.ReactNode;
  config?: WebSocketConfig;
  autoConnect?: boolean;
}

export function WebSocketProvider({
  children,
  config,
  autoConnect = true,
}: WebSocketProviderProps) {
  const [client, setClient] = useState<WebSocketClient | null>(null);
  const [connectionState, setConnectionState] = useState<ConnectionState>(
    ConnectionState.Disconnected,
  );
  const [lastEvent, setLastEvent] = useState<WebSocketEvent | null>(null);
  const [clientId, setClientId] = useState<string | null>(null);
  const clientRef = useRef<WebSocketClient | null>(null);

  // Initialize WebSocket client
  useEffect(() => {
    const wsClient = new WebSocketClient(config);

    // Set up event listeners
    wsClient.on("connectionStateChange", (state: ConnectionState) => {
      setConnectionState(state);
      if (state === ConnectionState.Connected) {
        setClientId(wsClient.getClientId());
      } else if (state === ConnectionState.Disconnected) {
        setClientId(null);
      }
    });

    wsClient.on("message", (event: WebSocketEvent) => {
      setLastEvent(event);
      // Update client ID if received in system alert
      if (event.event_type === EventType.SystemAlert && (event.data as any)?.client_id) {
        setClientId((event.data as any).client_id);
      }
    });

    wsClient.on("error", (error: Error) => {
      console.error("WebSocket error:", error);
    });

    setClient(wsClient);
    clientRef.current = wsClient;

    // Auto-connect if enabled
    if (autoConnect) {
      wsClient.connect();
    }

    // Cleanup on unmount
    return () => {
      wsClient.disconnect();
      wsClient.removeAllListeners();
    };
  }, []); // Only run once on mount

  const connect = useCallback(() => {
    clientRef.current?.connect();
  }, []);

  const disconnect = useCallback(() => {
    clientRef.current?.disconnect();
  }, []);

  const subscribe = useCallback((request: SubscriptionRequest) => {
    clientRef.current?.subscribe(request);
  }, []);

  const send = useCallback((message: string | ArrayBuffer) => {
    clientRef.current?.send(message);
  }, []);

  const sendJSON = useCallback((data: any) => {
    clientRef.current?.sendJSON(data);
  }, []);

  const contextValue: WebSocketContextValue = {
    client,
    connectionState,
    lastEvent,
    connect,
    disconnect,
    subscribe,
    send,
    sendJSON,
    isConnected: connectionState === ConnectionState.Connected,
    clientId,
  };

  return (
    <WebSocketContext.Provider value={contextValue}>
      {children}
    </WebSocketContext.Provider>
  );
}

export function useWebSocketContext(): WebSocketContextValue {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error("useWebSocketContext must be used within a WebSocketProvider");
  }
  return context;
}
