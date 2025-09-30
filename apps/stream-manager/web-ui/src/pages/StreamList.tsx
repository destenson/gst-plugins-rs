import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  ColumnDef,
  FilterFn,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  RowSelectionState,
  SortingState,
  useReactTable,
  VisibilityState,
} from "@tanstack/react-table";
import Select, { MultiValue } from "react-select";
import {
  Activity,
  AlertCircle,
  CheckCircle,
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
  Circle,
  Clock,
  Download,
  Eye,
  FileDown,
  Filter,
  Grid,
  HardDrive,
  List,
  Loader2,
  MoreVertical,
  PauseCircle,
  Play,
  PlayCircle,
  Plus,
  RotateCw,
  Search,
  Settings,
  Square,
  StopCircle,
  Trash2,
  Video,
  Wifi,
  WifiOff,
} from "lucide-react";
import { format } from "date-fns";
import { useAPI } from "../contexts/APIContext.tsx";
import { useWebSocketSubscription } from "../lib/websocket/hooks.ts";
import { EventType } from "../lib/websocket/types.ts";
import type { Stream } from "../api/types/index.ts";
import type { WebSocketEvent } from "../lib/websocket/types.ts";
import { cn } from "../lib/utils.ts";

type ViewMode = "table" | "card";
type FilterOption = { value: string; label: string };

const statusColors: Record<string, { bg: string; text: string; icon: React.ReactNode }> = {
  active: {
    bg: "bg-green-100 dark:bg-green-900/20",
    text: "text-green-800 dark:text-green-300",
    icon: <CheckCircle className="h-4 w-4" />,
  },
  inactive: {
    bg: "bg-gray-100 dark:bg-gray-800",
    text: "text-gray-600 dark:text-gray-400",
    icon: <Circle className="h-4 w-4" />,
  },
  error: {
    bg: "bg-red-100 dark:bg-red-900/20",
    text: "text-red-800 dark:text-red-300",
    icon: <AlertCircle className="h-4 w-4" />,
  },
  initializing: {
    bg: "bg-blue-100 dark:bg-blue-900/20",
    text: "text-blue-800 dark:text-blue-300",
    icon: <Loader2 className="h-4 w-4 animate-spin" />,
  },
  starting: {
    bg: "bg-yellow-100 dark:bg-yellow-900/20",
    text: "text-yellow-800 dark:text-yellow-300",
    icon: <PlayCircle className="h-4 w-4" />,
  },
  stopping: {
    bg: "bg-orange-100 dark:bg-orange-900/20",
    text: "text-orange-800 dark:text-orange-300",
    icon: <StopCircle className="h-4 w-4" />,
  },
  restarting: {
    bg: "bg-purple-100 dark:bg-purple-900/20",
    text: "text-purple-800 dark:text-purple-300",
    icon: <RotateCw className="h-4 w-4 animate-spin" />,
  },
};

const healthIndicators: Record<string, { color: string; label: string }> = {
  good: { color: "text-green-500", label: "Healthy" },
  warning: { color: "text-yellow-500", label: "Warning" },
  error: { color: "text-red-500", label: "Error" },
};

function StatusBadge({ status }: { status: string }) {
  const config = statusColors[status] || statusColors.inactive;
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium",
        config.bg,
        config.text,
      )}
    >
      {config.icon}
      {status}
    </span>
  );
}

function HealthIndicator({ stream }: { stream: Stream }) {
  const health = useMemo(() => {
    if (stream.errors && stream.errors.length > 0) return "error";
    if (stream.metrics?.packets_lost && stream.metrics.packets_lost > 100) return "warning";
    if (stream.status === "active") return "good";
    return "error";
  }, [stream]);

  const config = healthIndicators[health];
  return (
    <span title={config.label}>
      <Activity className={cn("h-4 w-4", config.color)} />
    </span>
  );
}

function RecordingStatus({ stream }: { stream: Stream }) {
  if (!stream.recording?.enabled) return <span className="text-gray-400 text-sm">-</span>;

  const status = stream.recording.status || "stopped";
  const icons = {
    recording: <Circle className="h-3 w-3 text-red-500 fill-red-500 animate-pulse" />,
    paused: <PauseCircle className="h-3 w-3 text-yellow-500" />,
    stopped: <StopCircle className="h-3 w-3 text-gray-400" />,
  };

  return (
    <div className="flex items-center gap-1.5">
      {icons[status]}
      <span className="text-xs text-gray-600 dark:text-gray-400">{status}</span>
    </div>
  );
}

function QuickActions(
  { stream, onAction }: { stream: Stream; onAction: (action: string, id: string) => void },
) {
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  return (
    <div className="relative" ref={menuRef}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="p-1.5 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
      >
        <MoreVertical className="h-4 w-4" />
      </button>

      {open && (
        <div className="absolute right-0 z-10 mt-1 w-48 rounded-md shadow-lg bg-white dark:bg-gray-900 ring-1 ring-black ring-opacity-5">
          <div className="py-1">
            {stream.status === "active"
              ? (
                <button
                  type="button"
                  onClick={() => {
                    onAction("stop", stream.id);
                    setOpen(false);
                  }}
                  className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
                >
                  <Square className="h-4 w-4 mr-2" />
                  Stop Stream
                </button>
              )
              : (
                <button
                  type="button"
                  onClick={() => {
                    onAction("start", stream.id);
                    setOpen(false);
                  }}
                  className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
                >
                  <Play className="h-4 w-4 mr-2" />
                  Start Stream
                </button>
              )}

            <button
              type="button"
              onClick={() => {
                onAction("restart", stream.id);
                setOpen(false);
              }}
              className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
            >
              <RotateCw className="h-4 w-4 mr-2" />
              Restart Stream
            </button>

            {stream.recording?.enabled && stream.recording.status !== "recording"
              ? (
                <button
                  type="button"
                  onClick={() => {
                    onAction("startRecording", stream.id);
                    setOpen(false);
                  }}
                  className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
                >
                  <Circle className="h-4 w-4 mr-2 text-red-500" />
                  Start Recording
                </button>
              )
              : stream.recording?.enabled && stream.recording.status === "recording"
              ? (
                <button
                  type="button"
                  onClick={() => {
                    onAction("stopRecording", stream.id);
                    setOpen(false);
                  }}
                  className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
                >
                  <Square className="h-4 w-4 mr-2" />
                  Stop Recording
                </button>
              )
              : null}

            <button
              type="button"
              onClick={() => {
                onAction("preview", stream.id);
                setOpen(false);
              }}
              className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
            >
              <Eye className="h-4 w-4 mr-2" />
              Preview Stream
            </button>

            <button
              type="button"
              onClick={() => {
                onAction("settings", stream.id);
                setOpen(false);
              }}
              className="flex items-center w-full px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
            >
              <Settings className="h-4 w-4 mr-2" />
              Settings
            </button>

            <div className="border-t dark:border-gray-800">
              <button
                type="button"
                onClick={() => {
                  onAction("delete", stream.id);
                  setOpen(false);
                }}
                className="flex items-center w-full px-4 py-2 text-sm text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/10"
              >
                <Trash2 className="h-4 w-4 mr-2" />
                Delete Stream
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function StreamCard({ stream, selected, onSelect, onAction }: {
  stream: Stream;
  selected: boolean;
  onSelect: (checked: boolean) => void;
  onAction: (action: string, id: string) => void;
}) {
  const [previewHover, setPreviewHover] = useState(false);

  return (
    <div
      className={cn(
        "bg-white dark:bg-gray-900 rounded-lg shadow-sm border p-4 space-y-3 transition-all",
        selected ? "ring-2 ring-blue-500 border-blue-500" : "border-gray-200 dark:border-gray-800",
        "hover:shadow-md",
      )}
    >
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3">
          <input
            type="checkbox"
            checked={selected}
            onChange={(e) => onSelect(e.target.checked)}
            className="mt-1 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
          />
          <div>
            <button
              type="button"
              onClick={() => onAction("preview", stream.id)}
              className="font-medium text-gray-900 dark:text-white hover:text-blue-600 dark:hover:text-blue-400 text-left"
            >
              {stream.id}
            </button>
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1 truncate max-w-xs">
              {stream.source_url}
            </p>
          </div>
        </div>
        <QuickActions stream={stream} onAction={onAction} />
      </div>

      <div className="flex items-center gap-4 text-sm">
        <StatusBadge status={stream.status} />
        <HealthIndicator stream={stream} />
        <RecordingStatus stream={stream} />
      </div>

      {stream.metrics && (
        <div className="grid grid-cols-2 gap-2 pt-2 border-t dark:border-gray-800">
          <div>
            <span className="text-xs text-gray-500">Bitrate</span>
            <p className="text-sm font-medium">
              {(stream.metrics.bitrate / 1000).toFixed(1)} kbps
            </p>
          </div>
          <div>
            <span className="text-xs text-gray-500">Framerate</span>
            <p className="text-sm font-medium">{stream.metrics.framerate} fps</p>
          </div>
        </div>
      )}

      <div
        className="relative h-32 bg-gray-100 dark:bg-gray-800 rounded overflow-hidden cursor-pointer group"
        onMouseEnter={() => setPreviewHover(true)}
        onMouseLeave={() => setPreviewHover(false)}
        onClick={() => onAction("preview", stream.id)}
      >
        <div className="absolute inset-0 flex items-center justify-center">
          <Video className="h-8 w-8 text-gray-400" />
        </div>
        {previewHover && (
          <div className="absolute inset-0 bg-black/50 flex items-center justify-center">
            <Eye className="h-8 w-8 text-white" />
          </div>
        )}
      </div>
    </div>
  );
}

function AddStreamModal({ isOpen, onClose, onAdd }: {
  isOpen: boolean;
  onClose: () => void;
  onAdd: (data: any) => void;
}) {
  const [formData, setFormData] = useState({
    id: "",
    source_url: "",
    recording: {
      enabled: false,
      segment_duration: 600,
      retention_days: 7,
    },
    reconnect: {
      enabled: true,
      max_attempts: 10,
      backoff_ms: 5000,
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onAdd(formData);
    setFormData({
      id: "",
      source_url: "",
      recording: {
        enabled: false,
        segment_duration: 600,
        retention_days: 7,
      },
      reconnect: {
        enabled: true,
        max_attempts: 10,
        backoff_ms: 5000,
      },
    });
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex min-h-full items-center justify-center p-4">
        <div className="fixed inset-0 bg-black/30" onClick={onClose} />

        <div className="relative bg-white dark:bg-gray-900 rounded-lg shadow-xl max-w-lg w-full p-6">
          <h2 className="text-xl font-semibold mb-4">Add New Stream</h2>

          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label className="block text-sm font-medium mb-1">Stream ID</label>
              <input
                type="text"
                required
                value={formData.id}
                onChange={(e) => setFormData({ ...formData, id: e.target.value })}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800"
                placeholder="camera-01"
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-1">Source URL</label>
              <input
                type="text"
                required
                value={formData.source_url}
                onChange={(e) => setFormData({ ...formData, source_url: e.target.value })}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800"
                placeholder="rtsp://192.168.1.100:554/stream"
              />
            </div>

            <div className="space-y-2">
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={formData.recording.enabled}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      recording: { ...formData.recording, enabled: e.target.checked },
                    })}
                  className="rounded"
                />
                <span className="text-sm font-medium">Enable Recording</span>
              </label>

              {formData.recording.enabled && (
                <div className="ml-6 space-y-2">
                  <div>
                    <label className="block text-xs text-gray-600 dark:text-gray-400">
                      Segment Duration (seconds)
                    </label>
                    <input
                      type="number"
                      value={formData.recording.segment_duration}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          recording: {
                            ...formData.recording,
                            segment_duration: Number(e.target.value),
                          },
                        })}
                      className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-700 rounded"
                    />
                  </div>

                  <div>
                    <label className="block text-xs text-gray-600 dark:text-gray-400">
                      Retention Days
                    </label>
                    <input
                      type="number"
                      value={formData.recording.retention_days}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          recording: {
                            ...formData.recording,
                            retention_days: Number(e.target.value),
                          },
                        })}
                      className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-700 rounded"
                    />
                  </div>
                </div>
              )}
            </div>

            <div className="space-y-2">
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={formData.reconnect.enabled}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      reconnect: { ...formData.reconnect, enabled: e.target.checked },
                    })}
                  className="rounded"
                />
                <span className="text-sm font-medium">Auto-Reconnect</span>
              </label>

              {formData.reconnect.enabled && (
                <div className="ml-6 space-y-2">
                  <div>
                    <label className="block text-xs text-gray-600 dark:text-gray-400">
                      Max Attempts
                    </label>
                    <input
                      type="number"
                      value={formData.reconnect.max_attempts}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          reconnect: {
                            ...formData.reconnect,
                            max_attempts: Number(e.target.value),
                          },
                        })}
                      className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-700 rounded"
                    />
                  </div>

                  <div>
                    <label className="block text-xs text-gray-600 dark:text-gray-400">
                      Backoff (ms)
                    </label>
                    <input
                      type="number"
                      value={formData.reconnect.backoff_ms}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          reconnect: { ...formData.reconnect, backoff_ms: Number(e.target.value) },
                        })}
                      className="w-full px-2 py-1 text-sm border border-gray-300 dark:border-gray-700 rounded"
                    />
                  </div>
                </div>
              )}
            </div>

            <div className="flex justify-end gap-3 pt-4">
              <button
                type="button"
                onClick={onClose}
                className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-md"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
              >
                Add Stream
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}

export default function StreamList() {
  const { api } = useAPI();
  const navigate = useNavigate();
  const { events } = useWebSocketSubscription({
    event_types: [
      EventType.StreamAdded,
      EventType.StreamRemoved,
      EventType.StreamHealthChanged,
      EventType.RecordingStarted,
      EventType.RecordingStopped,
      EventType.StatisticsUpdate,
    ],
  });
  const [streams, setStreams] = useState<Stream[]>([]);
  const [loading, setLoading] = useState(true);
  const [viewMode, setViewMode] = useState<ViewMode>("table");
  const [showAddModal, setShowAddModal] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<MultiValue<FilterOption>>([]);
  const [recordingFilter, setRecordingFilter] = useState<FilterOption | null>(null);
  const [healthFilter, setHealthFilter] = useState<FilterOption | null>(null);
  const [sorting, setSorting] = useState<SortingState>([]);
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({});

  const searchTimeoutRef = useRef<number>();

  // Debounce search input
  useEffect(() => {
    if (searchTimeoutRef.current) {
      clearTimeout(searchTimeoutRef.current);
    }
    searchTimeoutRef.current = setTimeout(() => {
      setDebouncedSearch(searchQuery);
    }, 300);

    return () => {
      if (searchTimeoutRef.current) {
        clearTimeout(searchTimeoutRef.current);
      }
    };
  }, [searchQuery]);

  // Load streams
  const loadStreams = useCallback(async () => {
    try {
      setLoading(true);
      const response = await api.streams.list();
      setStreams(response.streams);
    } catch (error) {
      console.error("Failed to load streams:", error);
    } finally {
      setLoading(false);
    }
  }, [api]);

  useEffect(() => {
    loadStreams();
  }, [loadStreams]);

  // Handle WebSocket events for real-time updates
  useEffect(() => {
    const updateStream = (streamId: string, updates: Partial<Stream>) => {
      setStreams((prev) => prev.map((s) => s.id === streamId ? { ...s, ...updates } : s));
    };

    events.forEach((event: WebSocketEvent) => {
      switch (event.event_type) {
        case EventType.StreamAdded:
          loadStreams();
          break;
        case EventType.StreamRemoved:
          loadStreams();
          break;
        case EventType.StreamHealthChanged:
          if (event.stream_id && event.data?.health) {
            updateStream(event.stream_id, {
              status: event.data.health === "healthy"
                ? "active"
                : event.data.health === "unhealthy"
                ? "error"
                : "inactive",
            });
          }
          break;
        case EventType.RecordingStarted:
          if (event.stream_id) {
            updateStream(event.stream_id, {
              recording: {
                enabled: true,
                status: "recording",
                current_file: event.data?.filename,
              },
            });
          }
          break;
        case EventType.RecordingStopped:
          if (event.stream_id) {
            updateStream(event.stream_id, {
              recording: {
                enabled: true,
                status: "stopped",
              },
            });
          }
          break;
        case EventType.StatisticsUpdate:
          if (event.stream_id && event.data?.metrics) {
            updateStream(event.stream_id, {
              metrics: {
                bitrate: event.data.metrics.bitrate || 0,
                framerate: event.data.metrics.framerate || 0,
                packets_received: event.data.metrics.packets_received || 0,
                packets_lost: event.data.metrics.packets_lost || 0,
              },
            });
          }
          break;
      }
    });
  }, [events, loadStreams]);

  // Handle actions
  const handleAction = useCallback(async (action: string, streamId: string) => {
    try {
      switch (action) {
        case "start":
          await api.streams.start(streamId);
          break;
        case "stop":
          await api.streams.stop(streamId);
          break;
        case "restart":
          await api.streams.restart(streamId);
          break;
        case "delete":
          if (confirm(`Are you sure you want to delete stream ${streamId}?`)) {
            await api.streams.delete(streamId);
            loadStreams();
          }
          break;
        case "startRecording":
          await api.streams.startRecording(streamId);
          break;
        case "stopRecording":
          await api.streams.stopRecording(streamId);
          break;
        case "preview":
          navigate(`/streams/${streamId}`);
          break;
        case "settings":
          navigate(`/streams/${streamId}?tab=config`);
          break;
      }
    } catch (error) {
      console.error(`Failed to ${action} stream:`, error);
    }
  }, [api, loadStreams]);

  // Handle bulk actions
  const handleBulkAction = useCallback(async (action: string) => {
    const selectedIds = Object.keys(rowSelection).filter((id) => rowSelection[id]);
    if (selectedIds.length === 0) return;

    try {
      switch (action) {
        case "start":
          await Promise.all(selectedIds.map((id) => api.streams.start(id)));
          break;
        case "stop":
          await Promise.all(selectedIds.map((id) => api.streams.stop(id)));
          break;
        case "restart":
          await Promise.all(selectedIds.map((id) => api.streams.restart(id)));
          break;
        case "delete":
          if (confirm(`Are you sure you want to delete ${selectedIds.length} streams?`)) {
            await Promise.all(selectedIds.map((id) => api.streams.delete(id)));
            loadStreams();
          }
          break;
      }
      setRowSelection({});
    } catch (error) {
      console.error(`Failed bulk ${action}:`, error);
    }
  }, [api, rowSelection, loadStreams]);

  // Handle add stream
  const handleAddStream = useCallback(async (data: any) => {
    try {
      await api.streams.create(data);
      loadStreams();
    } catch (error) {
      console.error("Failed to add stream:", error);
    }
  }, [api, loadStreams]);

  // Export to CSV
  const exportToCSV = useCallback(() => {
    const headers = [
      "ID",
      "Source URL",
      "Status",
      "Recording",
      "Bitrate",
      "Framerate",
      "Created At",
    ];
    const rows = streams.map((stream) => [
      stream.id,
      stream.source_url,
      stream.status,
      stream.recording?.status || "Not Recording",
      stream.metrics?.bitrate || 0,
      stream.metrics?.framerate || 0,
      stream.created_at || "",
    ]);

    const csv = [headers, ...rows].map((row) => row.join(",")).join("\n");
    const blob = new Blob([csv], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `streams-${format(new Date(), "yyyy-MM-dd-HH-mm")}.csv`;
    link.click();
    URL.revokeObjectURL(url);
  }, [streams]);

  // Filter function
  const globalFilter: FilterFn<Stream> = useCallback((row, columnId, filterValue) => {
    const value = String(row.getValue(columnId)).toLowerCase();
    return value.includes(filterValue.toLowerCase());
  }, []);

  // Filtered data
  const filteredData = useMemo(() => {
    let data = [...streams];

    // Apply search filter
    if (debouncedSearch) {
      data = data.filter((stream) =>
        stream.id.toLowerCase().includes(debouncedSearch.toLowerCase()) ||
        stream.source_url.toLowerCase().includes(debouncedSearch.toLowerCase())
      );
    }

    // Apply status filter
    if (statusFilter.length > 0) {
      data = data.filter((stream) => statusFilter.some((filter) => filter.value === stream.status));
    }

    // Apply recording filter
    if (recordingFilter) {
      if (recordingFilter.value === "recording") {
        data = data.filter((stream) => stream.recording?.status === "recording");
      } else if (recordingFilter.value === "enabled") {
        data = data.filter((stream) => stream.recording?.enabled);
      } else if (recordingFilter.value === "disabled") {
        data = data.filter((stream) => !stream.recording?.enabled);
      }
    }

    // Apply health filter
    if (healthFilter) {
      data = data.filter((stream) => {
        const hasErrors = stream.errors && stream.errors.length > 0;
        const hasWarnings = stream.metrics?.packets_lost && stream.metrics.packets_lost > 100;

        if (healthFilter.value === "healthy") {
          return stream.status === "active" && !hasErrors && !hasWarnings;
        } else if (healthFilter.value === "warning") {
          return hasWarnings && !hasErrors;
        } else if (healthFilter.value === "error") {
          return hasErrors;
        }
        return true;
      });
    }

    return data;
  }, [streams, debouncedSearch, statusFilter, recordingFilter, healthFilter]);

  // Table columns
  const columns: ColumnDef<Stream>[] = useMemo(() => [
    {
      id: "select",
      header: ({ table }) => (
        <input
          type="checkbox"
          checked={table.getIsAllPageRowsSelected()}
          onChange={(e) => table.toggleAllPageRowsSelected(e.target.checked)}
          className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
        />
      ),
      cell: ({ row }) => (
        <input
          type="checkbox"
          checked={row.getIsSelected()}
          onChange={(e) => row.toggleSelected(e.target.checked)}
          className="rounded border-gray-300 text-blue-600 focus:ring-blue-500"
        />
      ),
      size: 40,
    },
    {
      accessorKey: "id",
      header: "Stream ID",
      cell: ({ row }) => (
        <button
          type="button"
          onClick={() => navigate(`/streams/${row.original.id}`)}
          className="font-medium text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300"
        >
          {row.original.id}
        </button>
      ),
    },
    {
      accessorKey: "source_url",
      header: "Source URL",
      cell: ({ row }) => (
        <div className="text-sm text-gray-600 dark:text-gray-400 truncate max-w-xs">
          {row.original.source_url}
        </div>
      ),
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => <StatusBadge status={row.original.status} />,
    },
    {
      id: "health",
      header: "Health",
      cell: ({ row }) => <HealthIndicator stream={row.original} />,
    },
    {
      id: "recording",
      header: "Recording",
      cell: ({ row }) => <RecordingStatus stream={row.original} />,
    },
    {
      id: "bitrate",
      header: "Bitrate",
      cell: ({ row }) => (
        <span className="text-sm">
          {row.original.metrics ? `${(row.original.metrics.bitrate / 1000).toFixed(1)} kbps` : "-"}
        </span>
      ),
    },
    {
      id: "fps",
      header: "FPS",
      cell: ({ row }) => (
        <span className="text-sm">
          {row.original.metrics ? `${row.original.metrics.framerate} fps` : "-"}
        </span>
      ),
    },
    {
      id: "uptime",
      header: "Uptime",
      cell: ({ row }) => {
        if (!row.original.last_connected) return <span className="text-sm text-gray-400">-</span>;
        const uptime = Date.now() - new Date(row.original.last_connected).getTime();
        const hours = Math.floor(uptime / (1000 * 60 * 60));
        const minutes = Math.floor((uptime % (1000 * 60 * 60)) / (1000 * 60));
        return (
          <span className="text-sm">
            {hours > 0 ? `${hours}h ` : ""}
            {minutes}m
          </span>
        );
      },
    },
    {
      id: "actions",
      header: "Actions",
      cell: ({ row }) => <QuickActions stream={row.original} onAction={handleAction} />,
      size: 60,
    },
  ], [handleAction]);

  const table = useReactTable({
    data: filteredData,
    columns,
    state: {
      sorting,
      rowSelection,
      columnVisibility,
    },
    onSortingChange: setSorting,
    onRowSelectionChange: setRowSelection,
    onColumnVisibilityChange: setColumnVisibility,
    getCoreRowModel: getCoreRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    globalFilterFn: globalFilter,
  });

  const selectedCount = Object.keys(rowSelection).filter((id) => rowSelection[id]).length;

  const statusOptions: FilterOption[] = [
    { value: "active", label: "Active" },
    { value: "inactive", label: "Inactive" },
    { value: "error", label: "Error" },
    { value: "initializing", label: "Initializing" },
  ];

  const recordingOptions: FilterOption[] = [
    { value: "recording", label: "Recording" },
    { value: "enabled", label: "Enabled" },
    { value: "disabled", label: "Disabled" },
  ];

  const healthOptions: FilterOption[] = [
    { value: "healthy", label: "Healthy" },
    { value: "warning", label: "Warning" },
    { value: "error", label: "Error" },
  ];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-blue-500" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Streams
          </h2>
          <p className="text-sm text-gray-600 dark:text-gray-400">
            {filteredData.length} of {streams.length} streams
          </p>
        </div>

        <button
          type="button"
          onClick={() => setShowAddModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
        >
          <Plus className="h-4 w-4" />
          Add Stream
        </button>
      </div>

      {/* Toolbar */}
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-sm border border-gray-200 dark:border-gray-800 p-4">
        <div className="flex flex-col lg:flex-row gap-4">
          {/* Search */}
          <div className="flex-1">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-gray-400" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Search streams..."
                className="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800"
              />
            </div>
          </div>

          {/* Filters */}
          <div className="flex gap-2">
            <Select
              isMulti
              options={statusOptions}
              value={statusFilter}
              onChange={(value) => setStatusFilter(value as MultiValue<FilterOption>)}
              placeholder="Status"
              className="min-w-[150px]"
              classNamePrefix="select"
              theme={(theme) => ({
                ...theme,
                colors: {
                  ...theme.colors,
                  primary: "#3b82f6",
                },
              })}
            />

            <Select
              isClearable
              options={recordingOptions}
              value={recordingFilter}
              onChange={setRecordingFilter}
              placeholder="Recording"
              className="min-w-[150px]"
              classNamePrefix="select"
              theme={(theme) => ({
                ...theme,
                colors: {
                  ...theme.colors,
                  primary: "#3b82f6",
                },
              })}
            />

            <Select
              isClearable
              options={healthOptions}
              value={healthFilter}
              onChange={setHealthFilter}
              placeholder="Health"
              className="min-w-[150px]"
              classNamePrefix="select"
              theme={(theme) => ({
                ...theme,
                colors: {
                  ...theme.colors,
                  primary: "#3b82f6",
                },
              })}
            />
          </div>

          {/* View Mode & Export */}
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setViewMode(viewMode === "table" ? "card" : "table")}
              className="p-2 border border-gray-300 dark:border-gray-700 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800"
              title={viewMode === "table" ? "Card View" : "Table View"}
            >
              {viewMode === "table" ? <Grid className="h-4 w-4" /> : <List className="h-4 w-4" />}
            </button>

            <button
              type="button"
              onClick={exportToCSV}
              className="p-2 border border-gray-300 dark:border-gray-700 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800"
              title="Export to CSV"
            >
              <FileDown className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* Bulk Actions */}
        {selectedCount > 0 && (
          <div className="mt-4 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-md flex items-center justify-between">
            <span className="text-sm font-medium text-blue-800 dark:text-blue-300">
              {selectedCount} stream{selectedCount !== 1 ? "s" : ""} selected
            </span>

            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => handleBulkAction("start")}
                className="px-3 py-1 text-sm bg-green-600 text-white rounded hover:bg-green-700"
              >
                Start All
              </button>
              <button
                type="button"
                onClick={() => handleBulkAction("stop")}
                className="px-3 py-1 text-sm bg-orange-600 text-white rounded hover:bg-orange-700"
              >
                Stop All
              </button>
              <button
                type="button"
                onClick={() => handleBulkAction("restart")}
                className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
              >
                Restart All
              </button>
              <button
                type="button"
                onClick={() => handleBulkAction("delete")}
                className="px-3 py-1 text-sm bg-red-600 text-white rounded hover:bg-red-700"
              >
                Delete All
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Content */}
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-sm border border-gray-200 dark:border-gray-800">
        {viewMode === "table"
          ? (
            <>
              {/* Table */}
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead className="bg-gray-50 dark:bg-gray-800 border-b dark:border-gray-700">
                    {table.getHeaderGroups().map((headerGroup) => (
                      <tr key={headerGroup.id}>
                        {headerGroup.headers.map((header) => (
                          <th
                            key={header.id}
                            className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider"
                          >
                            {header.isPlaceholder ? null : (
                              <div
                                className={cn(
                                  "flex items-center gap-2",
                                  header.column.getCanSort() &&
                                    "cursor-pointer select-none hover:text-gray-700 dark:hover:text-gray-200",
                                )}
                                onClick={header.column.getToggleSortingHandler()}
                              >
                                {flexRender(header.column.columnDef.header, header.getContext())}
                                {header.column.getCanSort() && (
                                  <span className="text-gray-400">
                                    {{
                                      asc: "↑",
                                      desc: "↓",
                                    }[header.column.getIsSorted() as string] ?? "↕"}
                                  </span>
                                )}
                              </div>
                            )}
                          </th>
                        ))}
                      </tr>
                    ))}
                  </thead>
                  <tbody className="divide-y dark:divide-gray-800">
                    {table.getRowModel().rows.map((row) => (
                      <tr
                        key={row.id}
                        className="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
                      >
                        {row.getVisibleCells().map((cell) => (
                          <td key={cell.id} className="px-4 py-3">
                            {flexRender(cell.column.columnDef.cell, cell.getContext())}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {/* Pagination */}
              <div className="px-4 py-3 border-t dark:border-gray-800 flex items-center justify-between">
                <div className="text-sm text-gray-700 dark:text-gray-300">
                  Showing{" "}
                  {table.getState().pagination.pageIndex * table.getState().pagination.pageSize + 1}
                  {" "}
                  to {Math.min(
                    (table.getState().pagination.pageIndex + 1) *
                      table.getState().pagination.pageSize,
                    filteredData.length,
                  )} of {filteredData.length} results
                </div>

                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={() => table.setPageIndex(0)}
                    disabled={!table.getCanPreviousPage()}
                    className="p-1.5 border border-gray-300 dark:border-gray-700 rounded disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 dark:hover:bg-gray-800"
                  >
                    <ChevronsLeft className="h-4 w-4" />
                  </button>
                  <button
                    type="button"
                    onClick={() => table.previousPage()}
                    disabled={!table.getCanPreviousPage()}
                    className="p-1.5 border border-gray-300 dark:border-gray-700 rounded disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 dark:hover:bg-gray-800"
                  >
                    <ChevronLeft className="h-4 w-4" />
                  </button>
                  <span className="px-3 py-1.5 text-sm">
                    Page {table.getState().pagination.pageIndex + 1} of {table.getPageCount()}
                  </span>
                  <button
                    type="button"
                    onClick={() => table.nextPage()}
                    disabled={!table.getCanNextPage()}
                    className="p-1.5 border border-gray-300 dark:border-gray-700 rounded disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 dark:hover:bg-gray-800"
                  >
                    <ChevronRight className="h-4 w-4" />
                  </button>
                  <button
                    type="button"
                    onClick={() => table.setPageIndex(table.getPageCount() - 1)}
                    disabled={!table.getCanNextPage()}
                    className="p-1.5 border border-gray-300 dark:border-gray-700 rounded disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-100 dark:hover:bg-gray-800"
                  >
                    <ChevronsRight className="h-4 w-4" />
                  </button>
                </div>
              </div>
            </>
          )
          : (
            /* Card View */
            <div className="p-4 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
              {table.getRowModel().rows.map((row) => (
                <StreamCard
                  key={row.id}
                  stream={row.original}
                  selected={row.getIsSelected()}
                  onSelect={(checked) => row.toggleSelected(checked)}
                  onAction={handleAction}
                />
              ))}
            </div>
          )}
      </div>

      {/* Add Stream Modal */}
      <AddStreamModal
        isOpen={showAddModal}
        onClose={() => setShowAddModal(false)}
        onAdd={handleAddStream}
      />
    </div>
  );
}
