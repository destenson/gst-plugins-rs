import React from "react";
import { useConnectionState } from "./hooks.ts";
import { ConnectionState } from "./types.ts";
import { cn } from "../utils.ts";

interface ConnectionStatusProps {
  className?: string;
  showDetails?: boolean;
  compact?: boolean;
}

export function ConnectionStatus({
  className,
  showDetails = false,
  compact = false,
}: ConnectionStatusProps) {
  const {
    currentState,
    isConnected,
    isConnecting,
    isReconnecting,
    hasError,
  } = useConnectionState();

  const getStatusColor = () => {
    switch (currentState) {
      case ConnectionState.Connected:
        return "bg-green-500";
      case ConnectionState.Connecting:
      case ConnectionState.Reconnecting:
        return "bg-yellow-500";
      case ConnectionState.Error:
        return "bg-red-500";
      default:
        return "bg-gray-500";
    }
  };

  const getStatusText = () => {
    switch (currentState) {
      case ConnectionState.Connected:
        return "Connected";
      case ConnectionState.Connecting:
        return "Connecting...";
      case ConnectionState.Reconnecting:
        return "Reconnecting...";
      case ConnectionState.Error:
        return "Error";
      case ConnectionState.Disconnected:
        return "Disconnected";
      default:
        return "Unknown";
    }
  };

  const getStatusIcon = () => {
    if (isConnected) {
      return (
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
        </svg>
      );
    }
    if (isConnecting || isReconnecting) {
      return (
        <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
          />
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
          />
        </svg>
      );
    }
    if (hasError) {
      return (
        <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
      );
    }
    return (
      <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M18.364 5.636a9 9 0 010 12.728m0 0l-2.829-2.829m2.829 2.829L21 21M15.536 8.464a5 5 0 010 7.072m0 0l-2.829-2.829m-4.243 2.829a4.978 4.978 0 01-1.414-2.83m-1.414 5.658a9 9 0 01-2.167-9.238m7.824 2.167a1 1 0 111.414 1.414m-1.414-1.414L3 3m8.293 8.293l1.414 1.414"
        />
      </svg>
    );
  };

  if (compact) {
    return (
      <div className={cn("flex items-center gap-1", className)}>
        <span className={cn("w-2 h-2 rounded-full", getStatusColor())} />
        <span className="text-xs text-gray-600 dark:text-gray-400">
          WS
        </span>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "flex items-center gap-2 px-3 py-1.5 rounded-lg",
        "bg-gray-100 dark:bg-gray-800",
        "border border-gray-200 dark:border-gray-700",
        className,
      )}
    >
      <div className="flex items-center gap-2">
        <span className={cn("w-2 h-2 rounded-full animate-pulse", getStatusColor())} />
        <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
          {getStatusText()}
        </span>
      </div>

      {showDetails && (
        <div className="flex items-center text-gray-500 dark:text-gray-400">
          {getStatusIcon()}
        </div>
      )}
    </div>
  );
}

interface ConnectionBadgeProps {
  className?: string;
}

export function ConnectionBadge({ className }: ConnectionBadgeProps) {
  const { isConnected, isReconnecting } = useConnectionState();

  if (!isConnected && !isReconnecting) {
    return null;
  }

  return (
    <div
      className={cn(
        "fixed bottom-4 right-4 z-50",
        "px-3 py-1.5 rounded-full",
        "text-xs font-medium",
        "shadow-lg backdrop-blur-sm",
        "transition-all duration-300",
        isConnected
          ? "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 border border-green-300 dark:border-green-700"
          : "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 border border-yellow-300 dark:border-yellow-700",
        className,
      )}
    >
      <div className="flex items-center gap-2">
        <span
          className={cn(
            "w-1.5 h-1.5 rounded-full",
            isConnected ? "bg-green-500" : "bg-yellow-500 animate-pulse",
          )}
        />
        {isConnected ? "Live" : "Reconnecting..."}
      </div>
    </div>
  );
}
