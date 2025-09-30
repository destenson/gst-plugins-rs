import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import LoadingSpinner from "../components/LoadingSpinner.tsx";
import clsx from "clsx";
import {
  Area,
  AreaChart,
  CartesianGrid,
  Cell,
  Legend,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import {
  Activity,
  AlertCircle,
  AlertTriangle,
  ArrowDownRight,
  ArrowRight,
  ArrowUpRight,
  CheckCircle,
  Cpu,
  Database,
  Download,
  Film,
  HardDrive,
  MemoryStick,
  PlayCircle,
  Plus,
  Radio,
  RefreshCw,
  Server,
  Settings,
  Wifi,
  XCircle,
} from "lucide-react";
import { useAPI } from "../contexts/APIContext.tsx";
import { useConnectionState, useWebSocketSubscription } from "../lib/websocket/hooks.ts";
import { EventType, StatisticsUpdateData, WebSocketEvent } from "../lib/websocket/types.ts";
import type { Stream, SystemStatus } from "../api/types/index.ts";
import { formatDistanceToNow } from "date-fns";

interface MetricCard {
  title: string;
  value: string | number;
  change?: number;
  changeType?: "increase" | "decrease" | "neutral";
  icon: React.ReactNode;
  status?: "healthy" | "warning" | "error";
  unit?: string;
}

interface RecentEvent {
  id: string;
  timestamp: string;
  type: EventType;
  streamId?: string;
  message: string;
  level: "info" | "warning" | "error";
}

const STORAGE_COLORS = ["#0088FE", "#00C49F", "#FFBB28", "#FF8042"];
const REFRESH_INTERVAL = 30000; // 30 seconds

export default function Dashboard() {
  const navigate = useNavigate();
  const { api } = useAPI();

  // Use WebSocket subscription for all events
  const { events } = useWebSocketSubscription({
    // Subscribe to all relevant events
    event_types: [
      EventType.StreamAdded,
      EventType.StreamRemoved,
      EventType.StreamHealthChanged,
      EventType.RecordingStarted,
      EventType.RecordingStopped,
      EventType.StatisticsUpdate,
      EventType.SystemAlert,
      EventType.ErrorOccurred,
    ],
  });

  // Get connection state
  const { isConnected: connected } = useConnectionState();

  const [loading, setLoading] = useState(true);
  const [systemStatus, setSystemStatus] = useState<SystemStatus | null>(null);
  const [streams, setStreams] = useState<Stream[]>([]);
  const [recentEvents, setRecentEvents] = useState<RecentEvent[]>([]);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [lastRefresh, setLastRefresh] = useState(new Date());
  const [cpuHistory, setCpuHistory] = useState<Array<{ time: string; value: number }>>([]);
  const [memoryHistory, setMemoryHistory] = useState<Array<{ time: string; value: number }>>([]);
  const [backendError, setBackendError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);

  // Fetch system status
  const fetchSystemStatus = useCallback(async () => {
    try {
      setBackendError(null);
      // Check if api has the health property (real API) or if it's a mock
      if ("health" in api && api.health) {
        const status = await api.health.getStatus();
        setSystemStatus(status);
      } else if ("client" in api) {
        // Direct client call for compatibility
        const status = await api.client.get<SystemStatus>("/api/status");
        setSystemStatus(status);
      }
      setRetryCount(0);
    } catch (error: any) {
      console.error("Failed to fetch system status:", error);
      setBackendError("Backend API is not available");
      setRetryCount((prev) => prev + 1);
      // Stop retrying after 3 attempts
      if (retryCount >= 3) {
        setAutoRefresh(false);
      }
    }
  }, [api, retryCount]);

  // Fetch streams
  const fetchStreams = useCallback(async () => {
    try {
      // Check if api has the streams property (real API) or if it's a mock
      if ("streams" in api && api.streams) {
        const response = await api.streams.list();
        setStreams(response?.streams || []);
      } else if ("client" in api) {
        // Direct client call for compatibility
        const response = await api.client.get<{ streams: Stream[] }>("/api/streams");
        setStreams(response?.streams || []);
      }
    } catch (error: any) {
      console.error("Failed to fetch streams:", error);
      // Don't set error again if already set by system status
      if (!backendError) {
        setBackendError("Backend API is not available");
      }
    }
  }, [api, backendError]);

  // Load initial data
  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      await Promise.all([fetchSystemStatus(), fetchStreams()]);
      setLastRefresh(new Date());
    } finally {
      setLoading(false);
    }
  }, [fetchSystemStatus, fetchStreams]);

  // Handle WebSocket events
  useEffect(() => {
    if (!events || events.length === 0) return;

    const latestEvent = events[events.length - 1];

    // Update recent events list
    setRecentEvents((prev) => {
      const newEvent: RecentEvent = {
        id: latestEvent.id,
        timestamp: latestEvent.timestamp,
        type: latestEvent.event_type,
        streamId: latestEvent.stream_id,
        message: getEventMessage(latestEvent),
        level: getEventLevel(latestEvent.event_type),
      };

      const updated = [newEvent, ...prev].slice(0, 10); // Keep only last 10 events
      return updated;
    });

    // Handle statistics updates
    if (latestEvent.event_type === EventType.StatisticsUpdate) {
      const stats = latestEvent.data as StatisticsUpdateData;

      // Update CPU and memory history
      const timeStr = new Date().toLocaleTimeString();
      setCpuHistory(
        (prev) => [...prev.slice(-19), { time: timeStr, value: stats.system.cpu_usage_percent }],
      );
      setMemoryHistory(
        (prev) => [...prev.slice(-19), { time: timeStr, value: stats.system.memory_usage_mb }],
      );

      // Update system status
      setSystemStatus((prev) =>
        prev
          ? {
            ...prev,
            cpu_usage: stats.system.cpu_usage_percent,
            memory_usage: stats.system.memory_usage_mb,
            disk_usage: {
              ...prev.disk_usage,
              used: Math.round(prev.disk_usage.total * stats.system.disk_usage_percent / 100),
            },
          }
          : null
      );
    }

    // Refresh data on relevant events
    if (
      [EventType.StreamAdded, EventType.StreamRemoved, EventType.StreamHealthChanged].includes(
        latestEvent.event_type,
      )
    ) {
      fetchStreams();
    }
  }, [events, fetchStreams]);

  // Store loadData in a ref to avoid dependency issues
  const loadDataRef = useRef(loadData);
  useEffect(() => {
    loadDataRef.current = loadData;
  }, [loadData]);

  // Auto-refresh
  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      loadDataRef.current();
    }, REFRESH_INTERVAL);

    return () => clearInterval(interval);
  }, [autoRefresh]);

  // Initial load - only run once
  useEffect(() => {
    loadDataRef.current();
  }, []); // Empty dependency array

  // Calculate metrics
  const metrics = useMemo(() => {
    const activeStreams = streams.filter((s) => s.status === "active").length;
    const recordingStreams = streams.filter((s) => s.recording?.status === "recording").length;
    const errorStreams = streams.filter((s) => s.status === "error").length;

    return {
      activeStreams,
      recordingStreams,
      errorStreams,
      totalStreams: streams.length,
      storageUsed: systemStatus?.disk_usage.used || 0,
      storageTotal: systemStatus?.disk_usage.total || 0,
      cpuUsage: systemStatus?.cpu_usage || 0,
      memoryUsage: systemStatus?.memory_usage || 0,
    };
  }, [streams, systemStatus]);

  // Storage chart data
  const storageData = useMemo(() => {
    if (!systemStatus || systemStatus.disk_usage.total === 0) return [];

    const used = systemStatus.disk_usage.used;
    const total = systemStatus.disk_usage.total;
    const free = total - used;

    return [
      { name: "Used", value: used, percentage: (used / total * 100).toFixed(1) },
      { name: "Free", value: free, percentage: (free / total * 100).toFixed(1) },
    ];
  }, [systemStatus]);

  // System health status
  const systemHealth = useMemo(() => {
    if (!systemStatus) return "unknown";
    if (metrics.cpuUsage > 80 || metrics.memoryUsage > 80 || metrics.errorStreams > 0) {
      return "warning";
    }
    if (!connected) return "error";
    return "healthy";
  }, [systemStatus, metrics, connected]);

  // Loading skeleton
  if (loading && !systemStatus) {
    return (
      <div className="space-y-6">
        <div className="animate-pulse">
          <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded w-1/4 mb-6"></div>
          <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
            {[...Array(4)].map((_, i) => (
              <div key={i} className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
                <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2 mb-2"></div>
                <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Backend Error Banner */}
      {backendError && (
        <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
          <div className="flex items-center">
            <AlertTriangle className="h-5 w-5 text-red-600 dark:text-red-400 mr-3" />
            <div className="flex-1">
              <h3 className="text-sm font-semibold text-red-800 dark:text-red-300">
                Backend Connection Issue
              </h3>
              <p className="text-sm text-red-700 dark:text-red-400 mt-1">
                Unable to connect to the stream manager backend. The service may be starting up or
                experiencing issues.
              </p>
              {retryCount >= 3 && (
                <p className="text-xs text-red-600 dark:text-red-500 mt-2">
                  Auto-refresh has been disabled after multiple failed attempts.
                  <button
                    type="button"
                    onClick={() => {
                      setRetryCount(0);
                      setAutoRefresh(true);
                      loadData();
                    }}
                    className="ml-2 underline hover:no-underline"
                  >
                    Retry now
                  </button>
                </p>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">Dashboard</h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            System overview and real-time statistics
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => setAutoRefresh(!autoRefresh)}
            className={clsx(
              "inline-flex items-center px-3 py-1.5 text-sm font-medium rounded-md transition-colors",
              autoRefresh
                ? "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
                : "bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300",
            )}
          >
            <RefreshCw className={clsx("w-4 h-4 mr-1.5", autoRefresh && "animate-spin")} />
            {autoRefresh ? "Auto-refresh ON" : "Auto-refresh OFF"}
          </button>
          <button
            type="button"
            onClick={loadData}
            className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-white bg-primary-600 rounded-md hover:bg-primary-700 transition-colors"
          >
            <RefreshCw className="w-4 h-4 mr-1.5" />
            Refresh Now
          </button>
          <button
            type="button"
            onClick={() => exportData()}
            className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 dark:bg-gray-800 dark:text-gray-200 dark:border-gray-600 dark:hover:bg-gray-700 transition-colors"
          >
            <Download className="w-4 h-4 mr-1.5" />
            Export
          </button>
        </div>
      </div>

      {/* System Status Overview */}
      <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
        <StatusCard
          title="System Health"
          value={systemHealth === "healthy"
            ? "Operational"
            : systemHealth === "warning"
            ? "Degraded"
            : "Critical"}
          icon={systemHealth === "healthy"
            ? <CheckCircle className="w-5 h-5" />
            : systemHealth === "warning"
            ? <AlertTriangle className="w-5 h-5" />
            : <XCircle className="w-5 h-5" />}
          status={systemHealth as "healthy" | "warning" | "error"}
          subtitle={connected ? "Connected" : "Disconnected"}
        />

        <StatusCard
          title="Active Streams"
          value={metrics.activeStreams}
          total={metrics.totalStreams}
          icon={<Radio className="w-5 h-5" />}
          status={metrics.activeStreams > 0 ? "healthy" : "warning"}
          trend={calculateTrend(metrics.activeStreams, metrics.totalStreams)}
        />

        <StatusCard
          title="Recording"
          value={metrics.recordingStreams}
          icon={<Film className="w-5 h-5" />}
          status={metrics.recordingStreams > 0 ? "healthy" : "neutral"}
          subtitle={`${metrics.recordingStreams} active`}
        />

        <StatusCard
          title="Storage"
          value={`${formatBytes(metrics.storageUsed)}`}
          subtitle={`of ${formatBytes(metrics.storageTotal)}`}
          icon={<HardDrive className="w-5 h-5" />}
          status={metrics.storageTotal > 0 && metrics.storageUsed / metrics.storageTotal > 0.8
            ? "warning"
            : "healthy"}
          percentage={metrics.storageTotal > 0
            ? (metrics.storageUsed / metrics.storageTotal * 100).toFixed(0)
            : "0"}
        />
      </div>

      {/* Main Content Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Stream Statistics */}
        <div className="lg:col-span-2 space-y-6">
          {/* Resource Usage Charts */}
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              Resource Usage
            </h3>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              {/* CPU Chart */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-gray-600 dark:text-gray-400">CPU Usage</span>
                  <span className="text-sm font-medium text-gray-900 dark:text-white">
                    {metrics.cpuUsage.toFixed(1)}%
                  </span>
                </div>
                <ResponsiveContainer width="100%" height={100}>
                  <AreaChart data={cpuHistory}>
                    <defs>
                      <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="#3B82F6" stopOpacity={0.3} />
                        <stop offset="95%" stopColor="#3B82F6" stopOpacity={0} />
                      </linearGradient>
                    </defs>
                    <Area
                      type="monotone"
                      dataKey="value"
                      stroke="#3B82F6"
                      fill="url(#cpuGradient)"
                      strokeWidth={2}
                    />
                  </AreaChart>
                </ResponsiveContainer>
              </div>

              {/* Memory Chart */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-gray-600 dark:text-gray-400">Memory Usage</span>
                  <span className="text-sm font-medium text-gray-900 dark:text-white">
                    {formatBytes(metrics.memoryUsage * 1024 * 1024)}
                  </span>
                </div>
                <ResponsiveContainer width="100%" height={100}>
                  <AreaChart data={memoryHistory}>
                    <defs>
                      <linearGradient id="memGradient" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="5%" stopColor="#10B981" stopOpacity={0.3} />
                        <stop offset="95%" stopColor="#10B981" stopOpacity={0} />
                      </linearGradient>
                    </defs>
                    <Area
                      type="monotone"
                      dataKey="value"
                      stroke="#10B981"
                      fill="url(#memGradient)"
                      strokeWidth={2}
                    />
                  </AreaChart>
                </ResponsiveContainer>
              </div>
            </div>
          </div>

          {/* Stream Health Overview */}
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Stream Health</h3>
              <Link
                to="/streams"
                className="text-sm text-primary-600 hover:text-primary-700 dark:text-primary-400 dark:hover:text-primary-300 flex items-center"
              >
                View all
                <ArrowRight className="w-4 h-4 ml-1" />
              </Link>
            </div>
            <div className="space-y-3">
              {streams.slice(0, 5).map((stream) => (
                <div
                  key={stream.id}
                  className="flex items-center justify-between py-2 border-b border-gray-200 dark:border-gray-700 last:border-0"
                >
                  <div className="flex items-center space-x-3">
                    <span
                      className={clsx(
                        "w-2 h-2 rounded-full",
                        stream.status === "active"
                          ? "bg-green-500"
                          : stream.status === "error"
                          ? "bg-red-500"
                          : "bg-yellow-500",
                      )}
                    />
                    <div>
                      <p className="text-sm font-medium text-gray-900 dark:text-white">
                        {stream.id}
                      </p>
                      <p className="text-xs text-gray-500 dark:text-gray-400">
                        {stream.metrics?.bitrate
                          ? `${(stream.metrics.bitrate / 1000).toFixed(1)} Kbps`
                          : "No data"}
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center space-x-2">
                    {stream.recording?.status === "recording" && (
                      <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200">
                        REC
                      </span>
                    )}
                    <span
                      className={clsx(
                        "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium",
                        stream.status === "active"
                          ? "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
                          : stream.status === "error"
                          ? "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
                          : "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
                      )}
                    >
                      {stream.status}
                    </span>
                  </div>
                </div>
              ))}
              {streams.length === 0 && (
                <p className="text-center text-gray-500 dark:text-gray-400 py-4">
                  No streams configured
                </p>
              )}
            </div>
          </div>
        </div>

        {/* Right Column */}
        <div className="space-y-6">
          {/* Storage Usage Chart */}
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              Storage Overview
            </h3>
            {storageData.length > 0
              ? (
                <ResponsiveContainer width="100%" height={200}>
                  <PieChart>
                    <Pie
                      data={storageData}
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={80}
                      paddingAngle={2}
                      dataKey="value"
                    >
                      {storageData.map((entry, index) => (
                        <Cell
                          key={`cell-${index}`}
                          fill={STORAGE_COLORS[index % STORAGE_COLORS.length]}
                        />
                      ))}
                    </Pie>
                    <Tooltip formatter={(value: number) => formatBytes(value)} />
                  </PieChart>
                </ResponsiveContainer>
              )
              : (
                <div className="h-[200px] flex items-center justify-center text-gray-500 dark:text-gray-400">
                  No data available
                </div>
              )}
            <div className="mt-4 space-y-2">
              {storageData.map((item, index) => (
                <div key={item.name} className="flex items-center justify-between">
                  <div className="flex items-center">
                    <span
                      className="w-3 h-3 rounded-full mr-2"
                      style={{ backgroundColor: STORAGE_COLORS[index % STORAGE_COLORS.length] }}
                    />
                    <span className="text-sm text-gray-600 dark:text-gray-400">{item.name}</span>
                  </div>
                  <span className="text-sm font-medium text-gray-900 dark:text-white">
                    {formatBytes(item.value)} ({item.percentage}%)
                  </span>
                </div>
              ))}
            </div>
          </div>

          {/* Recent Events */}
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Recent Events</h3>
              <span className="text-xs text-gray-500 dark:text-gray-400">Live</span>
            </div>
            <div className="space-y-2 max-h-[300px] overflow-y-auto">
              {recentEvents.map((event) => (
                <div
                  key={event.id}
                  className="flex items-start space-x-2 py-2 border-b border-gray-200 dark:border-gray-700 last:border-0"
                >
                  <span
                    className={clsx(
                      "mt-1 w-2 h-2 rounded-full flex-shrink-0",
                      event.level === "error"
                        ? "bg-red-500"
                        : event.level === "warning"
                        ? "bg-yellow-500"
                        : "bg-blue-500",
                    )}
                  />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm text-gray-900 dark:text-white break-words">
                      {event.message}
                    </p>
                    <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                      {formatDistanceToNow(new Date(event.timestamp), { addSuffix: true })}
                      {event.streamId && ` • ${event.streamId}`}
                    </p>
                  </div>
                </div>
              ))}
              {recentEvents.length === 0 && (
                <p className="text-center text-gray-500 dark:text-gray-400 py-4">
                  No recent events
                </p>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Quick Actions */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Quick Actions</h3>
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
          <QuickActionButton
            label="Add Stream"
            icon={<Plus className="w-5 h-5" />}
            onClick={() => navigate("/streams?action=add")}
          />
          <QuickActionButton
            label="Start Recording"
            icon={<PlayCircle className="w-5 h-5" />}
            onClick={() => navigate("/streams?action=record")}
          />
          <QuickActionButton
            label="View Recordings"
            icon={<Film className="w-5 h-5" />}
            onClick={() => navigate("/recordings")}
          />
          <QuickActionButton
            label="System Metrics"
            icon={<Activity className="w-5 h-5" />}
            onClick={() => navigate("/metrics")}
          />
          <QuickActionButton
            label="Configuration"
            icon={<Settings className="w-5 h-5" />}
            onClick={() => navigate("/configuration")}
          />
          <QuickActionButton
            label="Database"
            icon={<Database className="w-5 h-5" />}
            onClick={() => navigate("/database")}
          />
        </div>
      </div>

      {/* Last Refresh */}
      <div className="text-center text-sm text-gray-500 dark:text-gray-400">
        Last refreshed: {lastRefresh.toLocaleTimeString()}
        {autoRefresh && <span className="ml-2">(Auto-refreshing every 30s)</span>}
      </div>
    </div>
  );
}

// Status Card Component
function StatusCard({
  title,
  value,
  subtitle,
  icon,
  status = "neutral",
  trend,
  total,
  percentage,
}: {
  title: string;
  value: string | number;
  subtitle?: string;
  icon: React.ReactNode;
  status?: "healthy" | "warning" | "error" | "neutral";
  trend?: "up" | "down" | "neutral";
  total?: number;
  percentage?: string;
}) {
  const statusColors = {
    healthy: "text-green-600 bg-green-100 dark:text-green-400 dark:bg-green-900",
    warning: "text-yellow-600 bg-yellow-100 dark:text-yellow-400 dark:bg-yellow-900",
    error: "text-red-600 bg-red-100 dark:text-red-400 dark:bg-red-900",
    neutral: "text-gray-600 bg-gray-100 dark:text-gray-400 dark:bg-gray-700",
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium text-gray-500 dark:text-gray-400">{title}</span>
        <span className={clsx("p-1.5 rounded-lg", statusColors[status])}>
          {icon}
        </span>
      </div>
      <div className="flex items-baseline justify-between">
        <div>
          <span className="text-2xl font-bold text-gray-900 dark:text-white">
            {value}
            {total !== undefined && (
              <span className="text-base font-normal text-gray-500 dark:text-gray-400">
                /{total}
              </span>
            )}
          </span>
          {subtitle && <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{subtitle}</p>}
        </div>
        {(trend || percentage) && (
          <div className="text-right">
            {trend && (
              <span
                className={clsx(
                  "inline-flex items-center text-sm font-medium",
                  trend === "up"
                    ? "text-green-600 dark:text-green-400"
                    : trend === "down"
                    ? "text-red-600 dark:text-red-400"
                    : "text-gray-500 dark:text-gray-400",
                )}
              >
                {trend === "up" && <ArrowUpRight className="w-4 h-4" />}
                {trend === "down" && <ArrowDownRight className="w-4 h-4" />}
              </span>
            )}
            {percentage && (
              <span className="text-sm text-gray-500 dark:text-gray-400">{percentage}%</span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// Quick Action Button Component
function QuickActionButton({
  label,
  icon,
  onClick,
}: {
  label: string;
  icon: React.ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex flex-col items-center justify-center p-4 text-gray-700 bg-gray-50 border border-gray-200 rounded-lg hover:bg-gray-100 dark:bg-gray-700 dark:text-gray-200 dark:border-gray-600 dark:hover:bg-gray-600 transition-colors"
    >
      {icon}
      <span className="mt-2 text-xs font-medium">{label}</span>
    </button>
  );
}

// Helper Functions
function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function calculateTrend(current: number, previous: number): "up" | "down" | "neutral" {
  if (current > previous) return "up";
  if (current < previous) return "down";
  return "neutral";
}

function getEventMessage(event: WebSocketEvent): string {
  switch (event.event_type) {
    case EventType.StreamAdded:
      return `Stream ${event.stream_id} added`;
    case EventType.StreamRemoved:
      return `Stream ${event.stream_id} removed`;
    case EventType.StreamHealthChanged:
      return `Stream ${event.stream_id} health changed to ${event.data.health}`;
    case EventType.RecordingStarted:
      return `Recording started for ${event.stream_id}`;
    case EventType.RecordingStopped:
      return `Recording stopped for ${event.stream_id}`;
    case EventType.StatisticsUpdate:
      return "Statistics updated";
    case EventType.SystemAlert:
      return event.data.message || "System alert";
    case EventType.ConfigChanged:
      return `Configuration changed: ${event.data.section}`;
    case EventType.ErrorOccurred:
      return `Error: ${event.data.error}`;
    default:
      return "Unknown event";
  }
}

function getEventLevel(type: EventType): "info" | "warning" | "error" {
  switch (type) {
    case EventType.ErrorOccurred:
      return "error";
    case EventType.StreamHealthChanged:
    case EventType.SystemAlert:
      return "warning";
    default:
      return "info";
  }
}

function exportData(): void {
  // TODO: Implement data export functionality
  console.log("Exporting dashboard data...");
  const data = {
    timestamp: new Date().toISOString(),
    // Add relevant data here
  };

  const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `dashboard-export-${Date.now()}.json`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
