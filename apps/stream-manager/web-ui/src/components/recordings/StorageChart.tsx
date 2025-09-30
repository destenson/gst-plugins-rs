import React, { useMemo } from "react";
import { CircleStackIcon, ServerStackIcon } from "@heroicons/react/24/outline";

interface StorageChartProps {
  totalSize: number;
  recordings: Array<{
    size: number;
    stream_id?: string;
  }>;
  streams: Array<{
    id: string;
    status: string;
  }>;
}

export default function StorageChart({ totalSize, recordings, streams }: StorageChartProps) {
  const { storageByStream, maxDiskSize } = useMemo(() => {
    const byStream = new Map<string, number>();

    recordings.forEach((recording) => {
      const streamId = recording.stream_id || "unknown";
      byStream.set(streamId, (byStream.get(streamId) || 0) + recording.size);
    });

    // Assume max disk size is 100GB for demo (should come from config)
    const maxSize = 100 * 1024 * 1024 * 1024; // 100GB in bytes

    return {
      storageByStream: Array.from(byStream.entries()).sort((a, b) => b[1] - a[1]),
      maxDiskSize: maxSize,
    };
  }, [recordings]);

  const formatSize = (bytes: number): string => {
    const units = ["B", "KB", "MB", "GB", "TB"];
    let size = bytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }

    return `${size.toFixed(2)} ${units[unitIndex]}`;
  };

  const getPercentage = (size: number, total: number): number => {
    if (total === 0) return 0;
    return (size / total) * 100;
  };

  // Stream colors
  const streamColors = [
    "bg-blue-500",
    "bg-green-500",
    "bg-purple-500",
    "bg-yellow-500",
    "bg-pink-500",
    "bg-indigo-500",
    "bg-red-500",
    "bg-orange-500",
  ];

  const getStreamColor = (index: number) => {
    return streamColors[index % streamColors.length];
  };

  const usagePercentage = getPercentage(totalSize, maxDiskSize);
  const usageColor = usagePercentage > 90
    ? "bg-red-500"
    : usagePercentage > 75
    ? "bg-yellow-500"
    : usagePercentage > 50
    ? "bg-blue-500"
    : "bg-green-500";

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white flex items-center">
          <CircleStackIcon className="h-5 w-5 mr-2" />
          Storage Usage
        </h3>
        <span className="text-sm text-gray-500 dark:text-gray-400">
          {formatSize(totalSize)} / {formatSize(maxDiskSize)}
        </span>
      </div>

      {/* Overall Usage Bar */}
      <div className="mb-6">
        <div className="flex justify-between text-sm mb-2">
          <span className="text-gray-600 dark:text-gray-400">Total Used</span>
          <span className="font-medium text-gray-900 dark:text-white">
            {usagePercentage.toFixed(1)}%
          </span>
        </div>
        <div className="w-full h-3 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
          <div
            className={`h-full ${usageColor} transition-all duration-500`}
            style={{ width: `${Math.min(usagePercentage, 100)}%` }}
          />
        </div>
        {usagePercentage > 75 && (
          <p className="mt-2 text-sm text-yellow-600 dark:text-yellow-400">
            ⚠ Storage is {usagePercentage > 90 ? "critically" : ""}{" "}
            low. Consider deleting old recordings.
          </p>
        )}
      </div>

      {/* Usage by Stream */}
      <div>
        <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
          Storage by Stream
        </h4>
        <div className="space-y-3">
          {storageByStream.length === 0
            ? (
              <p className="text-sm text-gray-500 dark:text-gray-400">
                No recordings yet
              </p>
            )
            : (
              storageByStream.slice(0, 5).map(([streamId, size], index) => {
                const percentage = getPercentage(size, totalSize);
                const stream = streams.find((s) => s.id === streamId);

                return (
                  <div key={streamId} className="flex items-center space-x-3">
                    {/* Stream Status Indicator */}
                    <div className="flex-shrink-0">
                      <div
                        className={`h-2 w-2 rounded-full ${
                          stream?.status === "active"
                            ? "bg-green-500"
                            : stream?.status === "error"
                            ? "bg-red-500"
                            : "bg-gray-500"
                        }`}
                      />
                    </div>

                    {/* Stream Name */}
                    <div className="flex-shrink-0 w-32">
                      <span className="text-sm font-medium text-gray-900 dark:text-white truncate block">
                        {streamId}
                      </span>
                    </div>

                    {/* Usage Bar */}
                    <div className="flex-1">
                      <div className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                        <div
                          className={`h-full ${getStreamColor(index)}`}
                          style={{ width: `${percentage}%` }}
                        />
                      </div>
                    </div>

                    {/* Size */}
                    <div className="flex-shrink-0 text-right">
                      <span className="text-sm text-gray-600 dark:text-gray-400">
                        {formatSize(size)}
                      </span>
                      <span className="text-xs text-gray-500 dark:text-gray-500 ml-1">
                        ({percentage.toFixed(1)}%)
                      </span>
                    </div>
                  </div>
                );
              })
            )}

          {storageByStream.length > 5 && (
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-2">
              And {storageByStream.length - 5} more streams...
            </p>
          )}
        </div>
      </div>

      {/* Storage Stats */}
      <div className="grid grid-cols-3 gap-4 mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
        <div className="text-center">
          <p className="text-2xl font-semibold text-gray-900 dark:text-white">
            {recordings.length}
          </p>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
            Total Recordings
          </p>
        </div>
        <div className="text-center">
          <p className="text-2xl font-semibold text-gray-900 dark:text-white">
            {storageByStream.length}
          </p>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
            Streams with Data
          </p>
        </div>
        <div className="text-center">
          <p className="text-2xl font-semibold text-gray-900 dark:text-white">
            {formatSize(maxDiskSize - totalSize)}
          </p>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
            Free Space
          </p>
        </div>
      </div>
    </div>
  );
}
