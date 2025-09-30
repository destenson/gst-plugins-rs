import React, { useEffect, useRef, useState } from "react";
import { useConnectionState, useWebSocketContext } from "./hooks.ts";
import { ConnectionState, EventType, WebSocketEvent } from "./types.ts";
import { cn } from "../utils.ts";

interface DebugPanelProps {
  className?: string;
  maxEvents?: number;
  defaultOpen?: boolean;
}

export function WebSocketDebugPanel({
  className,
  maxEvents = 100,
  defaultOpen = false,
}: DebugPanelProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  const [events, setEvents] = useState<Array<{ event: WebSocketEvent; timestamp: Date }>>([]);
  const [filter, setFilter] = useState<EventType | "all">("all");
  const [autoScroll, setAutoScroll] = useState(true);
  const eventsEndRef = useRef<HTMLDivElement>(null);
  const { client, clientId } = useWebSocketContext();
  const { currentState, stateHistory } = useConnectionState() as {
    currentState: ConnectionState;
    stateHistory: Array<{ state: ConnectionState; timestamp: Date }>;
  };

  // Listen for all events
  useEffect(() => {
    if (!client) return;

    const handleEvent = (event: WebSocketEvent) => {
      setEvents((prev) => {
        const newEvents = [...prev, { event, timestamp: new Date() }];
        return newEvents.slice(-maxEvents);
      });
    };

    client.on("message", handleEvent);

    return () => {
      client.removeListener("message", handleEvent);
    };
  }, [client, maxEvents]);

  // Auto-scroll to bottom
  useEffect(() => {
    if (autoScroll && eventsEndRef.current) {
      eventsEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [events, autoScroll]);

  const filteredEvents = filter === "all"
    ? events
    : events.filter((e) => e.event.event_type === filter);

  const clearEvents = () => setEvents([]);

  const getEventColor = (eventType: EventType) => {
    switch (eventType) {
      case EventType.StreamAdded:
      case EventType.RecordingStarted:
        return "text-green-600 dark:text-green-400";
      case EventType.StreamRemoved:
      case EventType.RecordingStopped:
        return "text-yellow-600 dark:text-yellow-400";
      case EventType.ErrorOccurred:
        return "text-red-600 dark:text-red-400";
      case EventType.SystemAlert:
        return "text-blue-600 dark:text-blue-400";
      case EventType.StatisticsUpdate:
        return "text-gray-600 dark:text-gray-400";
      default:
        return "text-gray-700 dark:text-gray-300";
    }
  };

  const getStateColor = (state: ConnectionState) => {
    switch (state) {
      case ConnectionState.Connected:
        return "text-green-600";
      case ConnectionState.Connecting:
      case ConnectionState.Reconnecting:
        return "text-yellow-600";
      case ConnectionState.Error:
        return "text-red-600";
      default:
        return "text-gray-600";
    }
  };

  if (!isOpen) {
    return (
      <button
        onClick={() => setIsOpen(true)}
        className={cn(
          "fixed bottom-4 left-4 z-40",
          "px-3 py-2 rounded-lg",
          "bg-gray-900 text-white",
          "hover:bg-gray-800",
          "transition-colors",
          "text-xs font-mono",
          className,
        )}
      >
        WS Debug
      </button>
    );
  }

  return (
    <div
      className={cn(
        "fixed bottom-4 left-4 z-40",
        "w-96 max-h-96",
        "bg-white dark:bg-gray-900",
        "border border-gray-200 dark:border-gray-700",
        "rounded-lg shadow-xl",
        "overflow-hidden",
        className,
      )}
    >
      {/* Header */}
      <div className="px-4 py-2 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold">WebSocket Debug</h3>
            <span className={cn("text-xs", getStateColor(currentState))}>
              {currentState}
            </span>
          </div>
          <button
            onClick={() => setIsOpen(false)}
            className="text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
          >
            <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
        {clientId && (
          <div className="text-xs text-gray-500 mt-1">
            Client ID: {clientId}
          </div>
        )}
      </div>

      {/* Controls */}
      <div className="px-4 py-2 bg-gray-50 dark:bg-gray-800/50 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center justify-between">
          <select
            value={filter}
            onChange={(e) => setFilter(e.target.value as EventType | "all")}
            className="text-xs px-2 py-1 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800"
          >
            <option value="all">All Events</option>
            {Object.values(EventType).map((type) => (
              <option key={type as string} value={type as string}>{type as string}</option>
            ))}
          </select>
          <div className="flex items-center gap-2">
            <label className="flex items-center gap-1 text-xs">
              <input
                type="checkbox"
                checked={autoScroll}
                onChange={(e) => setAutoScroll(e.target.checked)}
                className="w-3 h-3"
              />
              Auto-scroll
            </label>
            <button
              onClick={clearEvents}
              className="text-xs px-2 py-1 rounded bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600"
            >
              Clear
            </button>
          </div>
        </div>
        <div className="text-xs text-gray-500 mt-1">
          {filteredEvents.length} events ({events.length} total)
        </div>
      </div>

      {/* Events List */}
      <div className="h-64 overflow-y-auto p-2 font-mono text-xs">
        {filteredEvents.length === 0
          ? (
            <div className="text-center text-gray-500 py-4">
              No events yet
            </div>
          )
          : (
            <div className="space-y-1">
              {filteredEvents.map((item, index) => (
                <div
                  key={index}
                  className="p-2 bg-gray-50 dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-700"
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className={cn("font-semibold", getEventColor(item.event.event_type))}>
                      {item.event.event_type}
                    </span>
                    <span className="text-gray-500">
                      {item.timestamp.toLocaleTimeString()}
                    </span>
                  </div>
                  {item.event.stream_id && (
                    <div className="text-gray-600 dark:text-gray-400">
                      Stream: {item.event.stream_id}
                    </div>
                  )}
                  <details className="mt-1">
                    <summary className="cursor-pointer text-gray-500 hover:text-gray-700 dark:hover:text-gray-300">
                      Data
                    </summary>
                    <pre className="mt-1 p-1 bg-gray-100 dark:bg-gray-900 rounded text-xs overflow-x-auto">
                    {JSON.stringify(item.event.data, null, 2)}
                    </pre>
                  </details>
                </div>
              ))}
              <div ref={eventsEndRef} />
            </div>
          )}
      </div>

      {/* Connection History */}
      {stateHistory.length > 0 && (
        <details className="border-t border-gray-200 dark:border-gray-700">
          <summary className="px-4 py-2 text-xs cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800">
            Connection History ({stateHistory.length})
          </summary>
          <div className="max-h-32 overflow-y-auto p-2">
            {stateHistory.map((item, index) => (
              <div key={index} className="flex justify-between text-xs py-1">
                <span className={getStateColor(item.state)}>{item.state}</span>
                <span className="text-gray-500">
                  {item.timestamp.toLocaleTimeString()}
                </span>
              </div>
            ))}
          </div>
        </details>
      )}
    </div>
  );
}
