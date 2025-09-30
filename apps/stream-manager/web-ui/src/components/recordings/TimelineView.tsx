import React, { useMemo, useRef, useEffect, useState } from 'react';
import { FilmIcon, PlayIcon } from '@heroicons/react/24/outline';

interface TimelineViewProps {
  recordings: Array<{
    filename: string;
    created_at: string;
    duration: number;
    size: number;
    stream_id?: string;
  }>;
  onRecordingClick: (recording: any) => void;
}

export default function TimelineView({ recordings, onRecordingClick }: TimelineViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerWidth, setContainerWidth] = useState(0);

  useEffect(() => {
    const updateWidth = () => {
      if (containerRef.current) {
        setContainerWidth(containerRef.current.offsetWidth);
      }
    };

    updateWidth();
    globalThis.addEventListener('resize', updateWidth);
    return () => globalThis.removeEventListener('resize', updateWidth);
  }, []);

  // Calculate timeline range and grouping
  const { timelineData, hourMarkers, startTime, endTime } = useMemo(() => {
    if (recordings.length === 0) {
      const now = new Date();
      const start = new Date(now);
      start.setHours(0, 0, 0, 0);
      const end = new Date(now);
      end.setHours(23, 59, 59, 999);
      return { timelineData: [], hourMarkers: [], startTime: start, endTime: end };
    }

    // Find time range
    const times = recordings.map(r => new Date(r.created_at).getTime());
    const minTime = Math.min(...times);
    const maxTime = Math.max(...times);

    // Extend to full day boundaries
    const start = new Date(minTime);
    start.setHours(0, 0, 0, 0);
    const end = new Date(maxTime);
    end.setHours(23, 59, 59, 999);

    // Group recordings by stream
    const byStream = new Map<string, typeof recordings>();
    recordings.forEach(recording => {
      const streamId = recording.stream_id || 'unknown';
      if (!byStream.has(streamId)) {
        byStream.set(streamId, []);
      }
      byStream.get(streamId)!.push(recording);
    });

    // Sort recordings within each stream by time
    byStream.forEach(streamRecordings => {
      streamRecordings.sort((a, b) =>
        new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
      );
    });

    // Create hour markers
    const hours: Date[] = [];
    const current = new Date(start);
    while (current <= end) {
      hours.push(new Date(current));
      current.setHours(current.getHours() + 1);
    }

    return {
      timelineData: Array.from(byStream.entries()),
      hourMarkers: hours,
      startTime: start,
      endTime: end
    };
  }, [recordings]);

  // Calculate position and width for a recording
  const getRecordingPosition = (recording: typeof recordings[0]) => {
    if (containerWidth === 0) return { left: 0, width: 0 };

    const recordingStart = new Date(recording.created_at).getTime();
    const recordingEnd = recordingStart + (recording.duration * 1000);
    const totalDuration = endTime.getTime() - startTime.getTime();

    const left = ((recordingStart - startTime.getTime()) / totalDuration) * containerWidth;
    const width = ((recordingEnd - recordingStart) / totalDuration) * containerWidth;

    return { left, width: Math.max(width, 2) }; // Minimum width of 2px for visibility
  };

  // Format time for display
  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
  };

  const formatHour = (date: Date) => {
    return date.toLocaleTimeString('en-US', { hour: 'numeric', hour12: true });
  };

  const formatDuration = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);

    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    } else if (minutes > 0) {
      return `${minutes}m ${secs}s`;
    }
    return `${secs}s`;
  };

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

  // Stream colors
  const streamColors = [
    'bg-blue-500',
    'bg-green-500',
    'bg-purple-500',
    'bg-yellow-500',
    'bg-pink-500',
    'bg-indigo-500',
    'bg-red-500',
    'bg-orange-500'
  ];

  const getStreamColor = (index: number) => {
    return streamColors[index % streamColors.length];
  };

  if (recordings.length === 0) {
    return (
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-8">
        <div className="flex flex-col items-center justify-center">
          <FilmIcon className="h-12 w-12 text-gray-400 mb-4" />
          <p className="text-lg font-medium text-gray-900 dark:text-white">No recordings to display</p>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-2">
            Recordings will appear here in timeline view
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow">
      {/* Timeline Header */}
      <div className="p-4 border-b border-gray-200 dark:border-gray-700">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
          Recording Timeline
        </h3>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          {startTime.toLocaleDateString()} - {endTime.toLocaleDateString()}
        </p>
      </div>

      <div className="p-4">
        {/* Hour Markers */}
        <div className="relative h-8 mb-4" ref={containerRef}>
          <div className="absolute inset-0 border-b border-gray-200 dark:border-gray-700">
            {hourMarkers.map((hour, index) => {
              const position = (index / (hourMarkers.length - 1)) * 100;
              return (
                <div
                  key={hour.getTime()}
                  className="absolute top-0 bottom-0 border-l border-gray-200 dark:border-gray-700"
                  style={{ left: `${position}%` }}
                >
                  <span className="absolute -top-6 -left-6 text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap">
                    {formatHour(hour)}
                  </span>
                </div>
              );
            })}
          </div>
        </div>

        {/* Timeline Tracks */}
        <div className="space-y-3">
          {timelineData.map(([streamId, streamRecordings], streamIndex) => (
            <div key={streamId} className="relative">
              {/* Stream Label */}
              <div className="flex items-center mb-2">
                <div className={`h-3 w-3 rounded-full ${getStreamColor(streamIndex)} mr-2`} />
                <span className="text-sm font-medium text-gray-900 dark:text-white">
                  {streamId}
                </span>
                <span className="ml-2 text-xs text-gray-500 dark:text-gray-400">
                  ({streamRecordings.length} recordings)
                </span>
              </div>

              {/* Recording Blocks */}
              <div className="relative h-12 bg-gray-100 dark:bg-gray-900 rounded-lg overflow-hidden">
                {streamRecordings.map((recording, index) => {
                  const { left, width } = getRecordingPosition(recording);
                  return (
                    <button
                      key={recording.filename}
                      onClick={() => onRecordingClick(recording)}
                      className={`
                        absolute top-1 bottom-1 rounded
                        ${getStreamColor(streamIndex)} opacity-80 hover:opacity-100
                        transition-opacity cursor-pointer group
                      `}
                      style={{ left: `${left}px`, width: `${width}px` }}
                      title={`${recording.filename}\n${formatTime(new Date(recording.created_at))}\nDuration: ${formatDuration(recording.duration)}\nSize: ${formatSize(recording.size)}`}
                    >
                      {/* Show play icon on hover if block is wide enough */}
                      {width > 30 && (
                        <div className="flex items-center justify-center h-full opacity-0 group-hover:opacity-100 transition-opacity">
                          <PlayIcon className="h-5 w-5 text-white" />
                        </div>
                      )}
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </div>

        {/* Legend */}
        <div className="mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
          <div className="flex items-center justify-between text-sm">
            <div className="flex items-center space-x-4">
              <span className="text-gray-500 dark:text-gray-400">
                Total Recordings: <span className="font-medium text-gray-900 dark:text-white">{recordings.length}</span>
              </span>
              <span className="text-gray-500 dark:text-gray-400">
                Total Size: <span className="font-medium text-gray-900 dark:text-white">
                  {formatSize(recordings.reduce((sum, r) => sum + r.size, 0))}
                </span>
              </span>
              <span className="text-gray-500 dark:text-gray-400">
                Total Duration: <span className="font-medium text-gray-900 dark:text-white">
                  {formatDuration(recordings.reduce((sum, r) => sum + r.duration, 0))}
                </span>
              </span>
            </div>
            <div className="text-xs text-gray-500 dark:text-gray-400">
              Click on a recording block to play
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}