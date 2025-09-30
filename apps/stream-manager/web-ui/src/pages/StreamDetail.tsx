import React, { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { format } from 'date-fns';
import {
  Play,
  Square,
  RotateCw,
  Download,
  Settings,
  Circle,
  AlertCircle,
  CheckCircle,
  Clock,
  HardDrive,
  Activity,
  Wifi,
  WifiOff,
  Film,
  Server,
  Database,
  Maximize,
  Share2,
  Copy,
  ChevronRight,
  RefreshCw,
  Pause,
  SkipForward,
  Volume2,
  VolumeX,
  Loader2,
  Info,
  Save,
  X,
  Eye,
  EyeOff,
  FileText,
  BarChart3,
  Terminal,
  History,
  Trash2,
  ArrowLeft,
} from 'lucide-react';
import { useAPI } from '../contexts/APIContext.tsx';
import { useWebSocketSubscription } from '../lib/websocket/hooks.ts';
import { EventType } from '../lib/websocket/types.ts';
import type { Stream, DetailedMetrics, Recording } from '../api/types/index.ts';
import type { WebSocketEvent } from '../lib/websocket/types.ts';
import Breadcrumb from '../components/Breadcrumb.tsx';
import LoadingSpinner from '../components/LoadingSpinner.tsx';
import { cn } from '../lib/utils.ts';

// Video Player Component
function VideoPlayer({ streamId, sourceUrl, status }: { streamId: string; sourceUrl: string; status: string }) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isMuted, setIsMuted] = useState(true);
  const [volume, setVolume] = useState(1);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  // Generate HLS URL based on stream ID
  const videoUrl = useMemo(() => {
    if (status !== 'active') return '';
    // This assumes the backend provides HLS streams at this endpoint
    return `http://localhost:8080/streams/${streamId}/hls/index.m3u8`;
  }, [streamId, status]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video || !videoUrl) return;

    // For now, we'll use the native video element
    // In production, you'd want to use HLS.js for better compatibility
    video.src = videoUrl;

    const handleCanPlay = () => {
      setLoading(false);
      setError(null);
    };

    const handleError = () => {
      setLoading(false);
      setError('Failed to load stream');
    };

    video.addEventListener('canplay', handleCanPlay);
    video.addEventListener('error', handleError);

    return () => {
      video.removeEventListener('canplay', handleCanPlay);
      video.removeEventListener('error', handleError);
    };
  }, [videoUrl]);

  const togglePlay = () => {
    const video = videoRef.current;
    if (!video) return;

    if (isPlaying) {
      video.pause();
    } else {
      video.play().catch(err => {
        console.error('Play failed:', err);
        setError('Failed to play stream');
      });
    }
    setIsPlaying(!isPlaying);
  };

  const toggleMute = () => {
    const video = videoRef.current;
    if (!video) return;

    video.muted = !isMuted;
    setIsMuted(!isMuted);
  };

  const toggleFullscreen = () => {
    const container = videoRef.current?.parentElement;
    if (!container) return;

    if (!isFullscreen) {
      container.requestFullscreen();
    } else {
      document.exitFullscreen();
    }
    setIsFullscreen(!isFullscreen);
  };

  if (status !== 'active') {
    return (
      <div className="aspect-video bg-gray-900 rounded-lg flex items-center justify-center">
        <div className="text-center">
          <WifiOff className="h-12 w-12 text-gray-600 mx-auto mb-2" />
          <p className="text-gray-400">Stream is {status}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="relative bg-black rounded-lg overflow-hidden group">
      <video
        ref={videoRef}
        className="w-full aspect-video"
        autoPlay
        muted={isMuted}
        controls={false}
      />

      {loading && (
        <div className="absolute inset-0 flex items-center justify-center bg-gray-900">
          <Loader2 className="h-8 w-8 animate-spin text-blue-500" />
        </div>
      )}

      {error && (
        <div className="absolute inset-0 flex items-center justify-center bg-gray-900">
          <div className="text-center">
            <AlertCircle className="h-12 w-12 text-red-500 mx-auto mb-2" />
            <p className="text-gray-400">{error}</p>
          </div>
        </div>
      )}

      {/* Custom Controls */}
      <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent p-4 opacity-0 group-hover:opacity-100 transition-opacity">
        <div className="flex items-center gap-4">
          <button
            onClick={togglePlay}
            className="text-white hover:text-blue-400 transition-colors"
          >
            {isPlaying ? <Pause className="h-6 w-6" /> : <Play className="h-6 w-6" />}
          </button>

          <button
            onClick={toggleMute}
            className="text-white hover:text-blue-400 transition-colors"
          >
            {isMuted ? <VolumeX className="h-6 w-6" /> : <Volume2 className="h-6 w-6" />}
          </button>

          <div className="flex-1" />

          <button
            onClick={toggleFullscreen}
            className="text-white hover:text-blue-400 transition-colors"
          >
            <Maximize className="h-6 w-6" />
          </button>
        </div>
      </div>
    </div>
  );
}

// Status Badge Component
function StatusBadge({ status }: { status: string }) {
  const configs: Record<string, { bg: string; text: string; icon: React.ReactNode }> = {
    active: {
      bg: 'bg-green-100 dark:bg-green-900/20',
      text: 'text-green-800 dark:text-green-300',
      icon: <CheckCircle className="h-4 w-4" />,
    },
    inactive: {
      bg: 'bg-gray-100 dark:bg-gray-800',
      text: 'text-gray-600 dark:text-gray-400',
      icon: <Circle className="h-4 w-4" />,
    },
    error: {
      bg: 'bg-red-100 dark:bg-red-900/20',
      text: 'text-red-800 dark:text-red-300',
      icon: <AlertCircle className="h-4 w-4" />,
    },
  };

  const config = configs[status] || configs.inactive;
  return (
    <span className={cn(
      'inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-sm font-medium',
      config.bg,
      config.text
    )}>
      {config.icon}
      {status}
    </span>
  );
}

// Info Card Component
function InfoCard({ title, value, icon: Icon, trend }: {
  title: string;
  value: string | number;
  icon: React.ElementType;
  trend?: { value: number; isPositive: boolean };
}) {
  return (
    <div className="bg-white dark:bg-gray-900 p-4 rounded-lg border border-gray-200 dark:border-gray-800">
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm text-gray-600 dark:text-gray-400">{title}</p>
          <p className="text-2xl font-bold mt-1">{value}</p>
          {trend && (
            <p className={cn(
              'text-sm mt-1',
              trend.isPositive ? 'text-green-600' : 'text-red-600'
            )}>
              {trend.isPositive ? '↑' : '↓'} {Math.abs(trend.value)}%
            </p>
          )}
        </div>
        <Icon className="h-8 w-8 text-gray-400" />
      </div>
    </div>
  );
}

// Metrics Chart Component (simplified for now)
function MetricsChart({ title, data, color = 'blue' }: {
  title: string;
  data: number[];
  color?: string;
}) {
  const max = Math.max(...data);
  const normalized = data.map(v => (v / max) * 100);

  return (
    <div className="bg-white dark:bg-gray-900 p-4 rounded-lg border border-gray-200 dark:border-gray-800">
      <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">{title}</h3>
      <div className="h-32 flex items-end gap-1">
        {normalized.map((value, i) => (
          <div
            key={i}
            className={cn(
              'flex-1 rounded-t transition-all',
              color === 'blue' && 'bg-blue-500',
              color === 'green' && 'bg-green-500',
              color === 'yellow' && 'bg-yellow-500'
            )}
            style={{ height: `${value}%` }}
          />
        ))}
      </div>
    </div>
  );
}

// Event Log Item
function EventLogItem({ event }: { event: WebSocketEvent }) {
  const typeConfig: Record<string, { icon: React.ElementType; color: string }> = {
    'stream_added': { icon: Play, color: 'text-green-600' },
    'stream_removed': { icon: Square, color: 'text-red-600' },
    'stream_health_changed': { icon: Activity, color: 'text-yellow-600' },
    'recording_started': { icon: Circle, color: 'text-red-600' },
    'recording_stopped': { icon: Square, color: 'text-gray-600' },
    'error_occurred': { icon: AlertCircle, color: 'text-red-600' },
  };

  const config = typeConfig[event.event_type] || { icon: Info, color: 'text-gray-600' };
  const Icon = config.icon;

  // Get message from event data
  const getMessage = () => {
    if (event.data?.message) return event.data.message;
    switch (event.event_type) {
      case EventType.StreamAdded:
        return 'Stream added';
      case EventType.StreamRemoved:
        return 'Stream removed';
      case EventType.StreamHealthChanged:
        return `Stream health: ${event.data?.health || 'unknown'}`;
      case EventType.RecordingStarted:
        return 'Recording started';
      case EventType.RecordingStopped:
        return 'Recording stopped';
      case EventType.ErrorOccurred:
        return event.data?.error || 'Error occurred';
      default:
        return event.event_type.replace(/_/g, ' ');
    }
  };

  return (
    <div className="flex items-start gap-3 py-2">
      <Icon className={cn('h-4 w-4 mt-0.5', config.color)} />
      <div className="flex-1 min-w-0">
        <p className="text-sm text-gray-900 dark:text-white">{getMessage()}</p>
        <p className="text-xs text-gray-500 dark:text-gray-400">
          {format(new Date(event.timestamp), 'HH:mm:ss')}
        </p>
      </div>
    </div>
  );
}

// Recording History Item
function RecordingItem({ recording, onDownload, onDelete }: {
  recording: Recording;
  onDownload: () => void;
  onDelete: () => void;
}) {
  const formatSize = (bytes: number) => {
    const units = ['B', 'KB', 'MB', 'GB'];
    let size = bytes;
    let unitIndex = 0;
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }
    return `${size.toFixed(1)} ${units[unitIndex]}`;
  };

  const formatDuration = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m ${secs}s`;
  };

  return (
    <div className="flex items-center justify-between py-3 border-b dark:border-gray-800 last:border-0">
      <div className="min-w-0">
        <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
          {recording.filename}
        </p>
        <p className="text-xs text-gray-500 dark:text-gray-400">
          {format(new Date(recording.created_at), 'MMM d, yyyy HH:mm')} ·
          {formatSize(recording.size)} · {formatDuration(recording.duration)}
        </p>
      </div>
      <div className="flex items-center gap-2 ml-4">
        <button
          onClick={onDownload}
          className="p-1.5 text-gray-600 hover:text-blue-600 transition-colors"
          title="Download"
        >
          <Download className="h-4 w-4" />
        </button>
        <button
          onClick={onDelete}
          className="p-1.5 text-gray-600 hover:text-red-600 transition-colors"
          title="Delete"
        >
          <Trash2 className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}

// Main StreamDetail Component
export default function StreamDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { api } = useAPI();
  const [stream, setStream] = useState<Stream | null>(null);
  const [recordings, setRecordings] = useState<Recording[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'overview' | 'recordings' | 'config' | 'events'>('overview');
  const [configEditing, setConfigEditing] = useState(false);
  const [configData, setConfigData] = useState('');
  const [metricsHistory, setMetricsHistory] = useState({
    bitrate: Array(20).fill(0),
    framerate: Array(20).fill(0),
    latency: Array(20).fill(0),
  });

  // Subscribe to WebSocket events
  const { events } = useWebSocketSubscription({
    event_types: [
      EventType.StreamHealthChanged,
      EventType.RecordingStarted,
      EventType.RecordingStopped,
      EventType.StatisticsUpdate,
      EventType.ErrorOccurred,
    ],
    stream_ids: id ? [id] : [],
  });

  // Load stream data
  const loadStream = useCallback(async () => {
    if (!id) return;

    try {
      setLoading(true);
      const streamData = await api.streams.get(id);
      setStream(streamData);

      // Load recordings for this stream
      const recordingsData = await api.recordings.list({ stream_id: id });
      setRecordings(recordingsData.recordings);

      // Load config
      setConfigData(JSON.stringify(streamData, null, 2));
    } catch (error) {
      console.error('Failed to load stream:', error);
    } finally {
      setLoading(false);
    }
  }, [id, api]);

  useEffect(() => {
    loadStream();
  }, [loadStream]);

  // Handle WebSocket events
  useEffect(() => {
    events.forEach((event: WebSocketEvent) => {
      if (event.stream_id !== id) return;

      switch (event.event_type) {
        case EventType.StreamHealthChanged:
          setStream(prev => prev ? { ...prev, status: event.data?.health || prev.status } : prev);
          break;
        case EventType.StatisticsUpdate:
          if (event.data?.metrics) {
            setStream(prev => prev ? { ...prev, metrics: event.data.metrics } : prev);

            // Update metrics history
            setMetricsHistory(prev => ({
              bitrate: [...prev.bitrate.slice(1), event.data.metrics.bitrate / 1000],
              framerate: [...prev.framerate.slice(1), event.data.metrics.framerate],
              latency: [...prev.latency.slice(1), Math.random() * 100], // Placeholder
            }));
          }
          break;
        case EventType.RecordingStarted:
        case EventType.RecordingStopped:
          loadStream();
          break;
      }
    });
  }, [events, id, loadStream]);

  // Control handlers
  const handleStart = async () => {
    if (!id) return;
    try {
      await api.streams.start(id);
      loadStream();
    } catch (error) {
      console.error('Failed to start stream:', error);
    }
  };

  const handleStop = async () => {
    if (!id) return;
    try {
      await api.streams.stop(id);
      loadStream();
    } catch (error) {
      console.error('Failed to stop stream:', error);
    }
  };

  const handleRestart = async () => {
    if (!id) return;
    try {
      await api.streams.restart(id);
      loadStream();
    } catch (error) {
      console.error('Failed to restart stream:', error);
    }
  };

  const handleStartRecording = async () => {
    if (!id) return;
    try {
      await api.streams.startRecording(id);
      loadStream();
    } catch (error) {
      console.error('Failed to start recording:', error);
    }
  };

  const handleStopRecording = async () => {
    if (!id) return;
    try {
      await api.streams.stopRecording(id);
      loadStream();
    } catch (error) {
      console.error('Failed to stop recording:', error);
    }
  };

  const handleSaveConfig = async () => {
    if (!id) return;
    try {
      const config = JSON.parse(configData);
      await api.streams.update(id, config);
      setConfigEditing(false);
      loadStream();
    } catch (error) {
      console.error('Failed to save config:', error);
    }
  };

  const handleDelete = async () => {
    if (!id) return;
    if (!confirm(`Are you sure you want to delete stream ${id}?`)) return;

    try {
      await api.streams.delete(id);
      navigate('/streams');
    } catch (error) {
      console.error('Failed to delete stream:', error);
    }
  };

  const copyShareLink = () => {
    const url = `${window.location.origin}/streams/${id}`;
    navigator.clipboard.writeText(url);
  };

  if (loading) {
    return <LoadingSpinner />;
  }

  if (!stream) {
    return (
      <div className="text-center py-12">
        <AlertCircle className="h-12 w-12 text-gray-400 mx-auto mb-4" />
        <h2 className="text-xl font-semibold mb-2">Stream not found</h2>
        <p className="text-gray-600 dark:text-gray-400 mb-4">The stream "{id}" could not be found.</p>
        <Link
          to="/streams"
          className="inline-flex items-center gap-2 text-blue-600 hover:text-blue-700"
        >
          <ArrowLeft className="h-4 w-4" />
          Back to streams
        </Link>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Auto-generated Breadcrumb */}
      <Breadcrumb />

      {/* Header */}
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-sm border border-gray-200 dark:border-gray-800 p-6">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-gray-900 dark:text-white">{stream.id}</h1>
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">{stream.source_url}</p>
          </div>
          <div className="flex items-center gap-3">
            <StatusBadge status={stream.status} />
            <button
              onClick={copyShareLink}
              className="p-2 text-gray-600 hover:text-blue-600 transition-colors"
              title="Copy share link"
            >
              <Share2 className="h-5 w-5" />
            </button>
          </div>
        </div>

        {/* Control Buttons */}
        <div className="flex flex-wrap gap-3">
          {stream.status === 'active' ? (
            <button
              onClick={handleStop}
              className="flex items-center gap-2 px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700"
            >
              <Square className="h-4 w-4" />
              Stop Stream
            </button>
          ) : (
            <button
              onClick={handleStart}
              className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700"
            >
              <Play className="h-4 w-4" />
              Start Stream
            </button>
          )}

          <button
            onClick={handleRestart}
            className="flex items-center gap-2 px-4 py-2 border border-gray-300 dark:border-gray-700 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            <RotateCw className="h-4 w-4" />
            Restart
          </button>

          {stream.recording?.enabled && (
            <>
              {stream.recording.status === 'recording' ? (
                <button
                  onClick={handleStopRecording}
                  className="flex items-center gap-2 px-4 py-2 border border-gray-300 dark:border-gray-700 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800"
                >
                  <Square className="h-4 w-4" />
                  Stop Recording
                </button>
              ) : (
                <button
                  onClick={handleStartRecording}
                  className="flex items-center gap-2 px-4 py-2 border border-gray-300 dark:border-gray-700 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800"
                >
                  <Circle className="h-4 w-4 text-red-500" />
                  Start Recording
                </button>
              )}
            </>
          )}

          <div className="flex-1" />

          <button
            onClick={handleDelete}
            className="flex items-center gap-2 px-4 py-2 border border-red-300 text-red-600 rounded-md hover:bg-red-50 dark:hover:bg-red-900/10"
          >
            <Trash2 className="h-4 w-4" />
            Delete
          </button>
        </div>
      </div>

      {/* Video Preview */}
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-sm border border-gray-200 dark:border-gray-800 p-6">
        <h2 className="text-lg font-semibold mb-4">Live Preview</h2>
        <VideoPlayer streamId={stream.id} sourceUrl={stream.source_url} status={stream.status} />
      </div>

      {/* Tabs */}
      <div className="bg-white dark:bg-gray-900 rounded-lg shadow-sm border border-gray-200 dark:border-gray-800">
        <div className="border-b dark:border-gray-800">
          <nav className="flex -mb-px">
            <button
              onClick={() => setActiveTab('overview')}
              className={cn(
                'px-6 py-3 text-sm font-medium border-b-2 transition-colors',
                activeTab === 'overview'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white'
              )}
            >
              <BarChart3 className="inline h-4 w-4 mr-2" />
              Overview
            </button>
            <button
              onClick={() => setActiveTab('recordings')}
              className={cn(
                'px-6 py-3 text-sm font-medium border-b-2 transition-colors',
                activeTab === 'recordings'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white'
              )}
            >
              <Film className="inline h-4 w-4 mr-2" />
              Recordings
            </button>
            <button
              onClick={() => setActiveTab('config')}
              className={cn(
                'px-6 py-3 text-sm font-medium border-b-2 transition-colors',
                activeTab === 'config'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white'
              )}
            >
              <Settings className="inline h-4 w-4 mr-2" />
              Configuration
            </button>
            <button
              onClick={() => setActiveTab('events')}
              className={cn(
                'px-6 py-3 text-sm font-medium border-b-2 transition-colors',
                activeTab === 'events'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white'
              )}
            >
              <Terminal className="inline h-4 w-4 mr-2" />
              Events
            </button>
          </nav>
        </div>

        <div className="p-6">
          {activeTab === 'overview' && (
            <div className="space-y-6">
              {/* Info Cards */}
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                <InfoCard
                  title="Bitrate"
                  value={stream.metrics ? `${(stream.metrics.bitrate / 1000).toFixed(1)} kbps` : '0 kbps'}
                  icon={Activity}
                />
                <InfoCard
                  title="Framerate"
                  value={stream.metrics ? `${stream.metrics.framerate} fps` : '0 fps'}
                  icon={Film}
                />
                <InfoCard
                  title="Packets Lost"
                  value={stream.metrics?.packets_lost || 0}
                  icon={Wifi}
                  trend={stream.metrics?.packets_lost ? { value: 2.3, isPositive: false } : undefined}
                />
                <InfoCard
                  title="Uptime"
                  value={stream.last_connected ?
                    `${Math.floor((Date.now() - new Date(stream.last_connected).getTime()) / (1000 * 60))}m` :
                    'N/A'
                  }
                  icon={Clock}
                />
              </div>

              {/* Metrics Charts */}
              <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
                <MetricsChart title="Bitrate (kbps)" data={metricsHistory.bitrate} color="blue" />
                <MetricsChart title="Framerate (fps)" data={metricsHistory.framerate} color="green" />
                <MetricsChart title="Latency (ms)" data={metricsHistory.latency} color="yellow" />
              </div>

              {/* Stream Info */}
              <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
                <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">Stream Information</h3>
                <dl className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div>
                    <dt className="text-sm text-gray-600 dark:text-gray-400">Created</dt>
                    <dd className="text-sm font-medium">
                      {stream.created_at ? format(new Date(stream.created_at), 'PPpp') : 'Unknown'}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-sm text-gray-600 dark:text-gray-400">Last Connected</dt>
                    <dd className="text-sm font-medium">
                      {stream.last_connected ? format(new Date(stream.last_connected), 'PPpp') : 'Never'}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-sm text-gray-600 dark:text-gray-400">Recording</dt>
                    <dd className="text-sm font-medium">
                      {stream.recording?.enabled ? 'Enabled' : 'Disabled'}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-sm text-gray-600 dark:text-gray-400">Auto-Reconnect</dt>
                    <dd className="text-sm font-medium">
                      {stream.reconnect?.enabled ? 'Enabled' : 'Disabled'}
                    </dd>
                  </div>
                </dl>
              </div>
            </div>
          )}

          {activeTab === 'recordings' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold">Recording History</h3>
                <div className="text-sm text-gray-600 dark:text-gray-400">
                  {recordings.length} recordings
                </div>
              </div>

              {recordings.length === 0 ? (
                <div className="text-center py-8 text-gray-500">
                  <Film className="h-12 w-12 text-gray-400 mx-auto mb-2" />
                  <p>No recordings found for this stream</p>
                </div>
              ) : (
                <div className="divide-y dark:divide-gray-800">
                  {recordings.map((recording) => (
                    <RecordingItem
                      key={recording.filename}
                      recording={recording}
                      onDownload={() => {
                        // Download implementation
                        window.open(`http://localhost:8080/recordings/${recording.filename}/download`, '_blank');
                      }}
                      onDelete={async () => {
                        if (confirm(`Delete recording ${recording.filename}?`)) {
                          // Delete implementation
                          console.log('Delete:', recording.filename);
                        }
                      }}
                    />
                  ))}
                </div>
              )}
            </div>
          )}

          {activeTab === 'config' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold">Stream Configuration</h3>
                {configEditing ? (
                  <div className="flex gap-2">
                    <button
                      onClick={handleSaveConfig}
                      className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
                    >
                      <Save className="h-4 w-4" />
                      Save
                    </button>
                    <button
                      onClick={() => {
                        setConfigData(JSON.stringify(stream, null, 2));
                        setConfigEditing(false);
                      }}
                      className="flex items-center gap-2 px-4 py-2 border border-gray-300 dark:border-gray-700 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800"
                    >
                      <X className="h-4 w-4" />
                      Cancel
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => setConfigEditing(true)}
                    className="flex items-center gap-2 px-4 py-2 border border-gray-300 dark:border-gray-700 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800"
                  >
                    <Settings className="h-4 w-4" />
                    Edit
                  </button>
                )}
              </div>

              <div className="relative">
                <textarea
                  value={configData}
                  onChange={(e) => setConfigData(e.target.value)}
                  disabled={!configEditing}
                  className={cn(
                    'w-full h-96 p-4 font-mono text-sm rounded-lg border',
                    configEditing
                      ? 'bg-white dark:bg-gray-800 border-blue-500'
                      : 'bg-gray-50 dark:bg-gray-900 border-gray-200 dark:border-gray-800'
                  )}
                />
              </div>
            </div>
          )}

          {activeTab === 'events' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold">Event Log</h3>
                <button
                  onClick={() => {
                    // Refresh events
                    loadStream();
                  }}
                  className="p-2 text-gray-600 hover:text-blue-600 transition-colors"
                  title="Refresh"
                >
                  <RefreshCw className="h-4 w-4" />
                </button>
              </div>

              <div className="divide-y dark:divide-gray-800 max-h-96 overflow-y-auto">
                {events.length === 0 ? (
                  <div className="text-center py-8 text-gray-500">
                    <History className="h-12 w-12 text-gray-400 mx-auto mb-2" />
                    <p>No events recorded yet</p>
                  </div>
                ) : (
                  events.slice(-20).reverse().map((event, i) => (
                    <EventLogItem key={i} event={event} />
                  ))
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}