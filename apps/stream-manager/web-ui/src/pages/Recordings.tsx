import { useState, useEffect, useMemo } from 'react';
import { useAPI } from '../contexts/APIContext.tsx';
import LoadingSpinner from '../components/LoadingSpinner.tsx';
import {
  FilmIcon,
  TrashIcon,
  PlayIcon,
  ExclamationTriangleIcon,
  FolderIcon,
  MagnifyingGlassIcon,
  ArrowDownTrayIcon
} from '@heroicons/react/24/outline';
import RecordingPlayer from '../components/recordings/RecordingPlayer.tsx';
import CalendarView from '../components/recordings/CalendarView.tsx';
import TimelineView from '../components/recordings/TimelineView.tsx';
import StorageChart from '../components/recordings/StorageChart.tsx';
import DeleteConfirmationModal from '../components/recordings/DeleteConfirmationModal.tsx';
import BulkActionsToolbar from '../components/recordings/BulkActionsToolbar.tsx';
import type { Recording, Stream } from '../api/types/index.ts';

type ViewMode = 'list' | 'calendar' | 'timeline';
type SortField = 'filename' | 'size' | 'duration' | 'created_at';
type SortOrder = 'asc' | 'desc';

interface RecordingWithStream extends Recording {
  stream_id?: string;
  stream?: Stream;
}

export default function Recordings() {
  const { api } = useAPI();
  const [recordings, setRecordings] = useState<RecordingWithStream[]>([]);
  const [streams, setStreams] = useState<Stream[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [selectedRecordings, setSelectedRecordings] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedStream, setSelectedStream] = useState<string>('all');
  const [dateRange, setDateRange] = useState<{ start: Date | null; end: Date | null }>({
    start: null,
    end: null
  });
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortOrder, setSortOrder] = useState<SortOrder>('desc');
  const [playerOpen, setPlayerOpen] = useState(false);
  const [selectedRecording, setSelectedRecording] = useState<RecordingWithStream | null>(null);
  const [deleteModalOpen, setDeleteModalOpen] = useState(false);
  const [recordingToDelete, setRecordingToDelete] = useState<RecordingWithStream | null>(null);
  const [bulkDeleteMode, setBulkDeleteMode] = useState(false);
  const [totalSize, setTotalSize] = useState(0);
  const [totalDuration, setTotalDuration] = useState(0);

  // Fetch recordings and streams
  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true);
        setError(null);

        // Fetch both recordings and streams in parallel
        const [recordingsResponse, streamsResponse] = await Promise.all([
          api.recordings.list({
            start_date: dateRange.start?.toISOString(),
            end_date: dateRange.end?.toISOString()
          }),
          api.streams.list()
        ]);

        setStreams(streamsResponse.streams);

        // Enhance recordings with stream information
        const enhancedRecordings = recordingsResponse.recordings.map(rec => {
          const streamId = extractStreamIdFromPath(rec.path);
          const stream = streamsResponse.streams.find(s => s.id === streamId);
          return {
            ...rec,
            stream_id: streamId,
            stream
          };
        });

        setRecordings(enhancedRecordings);
        setTotalSize(recordingsResponse.total_size);
        setTotalDuration(recordingsResponse.total_duration);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load recordings');
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [api, dateRange]);

  // Extract stream ID from recording path
  const extractStreamIdFromPath = (path: string): string | undefined => {
    const match = path.match(/\/([^\/]+)\/[^\/]+$/);
    return match ? match[1] : undefined;
  };

  // Filter and sort recordings
  const filteredRecordings = useMemo(() => {
    let filtered = [...recordings];

    // Filter by search query
    if (searchQuery) {
      filtered = filtered.filter(rec =>
        rec.filename.toLowerCase().includes(searchQuery.toLowerCase()) ||
        rec.stream_id?.toLowerCase().includes(searchQuery.toLowerCase())
      );
    }

    // Filter by selected stream
    if (selectedStream !== 'all') {
      filtered = filtered.filter(rec => rec.stream_id === selectedStream);
    }

    // Sort recordings
    filtered.sort((a, b) => {
      let comparison = 0;

      switch (sortField) {
        case 'filename':
          comparison = a.filename.localeCompare(b.filename);
          break;
        case 'size':
          comparison = a.size - b.size;
          break;
        case 'duration':
          comparison = a.duration - b.duration;
          break;
        case 'created_at':
          comparison = new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
          break;
      }

      return sortOrder === 'asc' ? comparison : -comparison;
    });

    return filtered;
  }, [recordings, searchQuery, selectedStream, sortField, sortOrder]);

  // Toggle recording selection
  const toggleRecordingSelection = (filename: string) => {
    const newSelection = new Set(selectedRecordings);
    if (newSelection.has(filename)) {
      newSelection.delete(filename);
    } else {
      newSelection.add(filename);
    }
    setSelectedRecordings(newSelection);
  };

  // Select all recordings
  const selectAllRecordings = () => {
    if (selectedRecordings.size === filteredRecordings.length) {
      setSelectedRecordings(new Set());
    } else {
      setSelectedRecordings(new Set(filteredRecordings.map(r => r.filename)));
    }
  };

  // Handle recording play
  const handlePlay = (recording: RecordingWithStream) => {
    setSelectedRecording(recording);
    setPlayerOpen(true);
  };

  // Handle single recording download
  const handleDownload = async (recording: RecordingWithStream) => {
    try {
      const blob = await api.recordings.download(recording.filename);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = recording.filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Download failed:', err);
      alert('Failed to download recording');
    }
  };

  // Handle bulk download
  const handleBulkDownload = async () => {
    for (const filename of selectedRecordings) {
      const recording = recordings.find(r => r.filename === filename);
      if (recording) {
        await handleDownload(recording);
      }
    }
  };

  // Handle delete confirmation
  const handleDelete = (recording: RecordingWithStream) => {
    setRecordingToDelete(recording);
    setBulkDeleteMode(false);
    setDeleteModalOpen(true);
  };

  // Handle bulk delete
  const handleBulkDelete = () => {
    setBulkDeleteMode(true);
    setDeleteModalOpen(true);
  };

  // Confirm deletion
  const confirmDelete = async () => {
    try {
      if (bulkDeleteMode) {
        for (const filename of selectedRecordings) {
          await api.recordings.delete(filename);
        }
        setRecordings(prev => prev.filter(r => !selectedRecordings.has(r.filename)));
        setSelectedRecordings(new Set());
      } else if (recordingToDelete) {
        await api.recordings.delete(recordingToDelete.filename);
        setRecordings(prev => prev.filter(r => r.filename !== recordingToDelete.filename));
      }
      setDeleteModalOpen(false);
    } catch (err) {
      console.error('Delete failed:', err);
      alert('Failed to delete recording(s)');
    }
  };

  // Format file size
  const formatSize = (bytes: number): string => {
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let size = bytes;
    let unitIndex = 0;

    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }

    return `${size.toFixed(2)} ${units[unitIndex]}`;
  };

  // Format duration
  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);

    if (hours > 0) {
      return `${hours}h ${minutes}m ${secs}s`;
    } else if (minutes > 0) {
      return `${minutes}m ${secs}s`;
    } else {
      return `${secs}s`;
    }
  };

  // Format date
  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    return date.toLocaleString();
  };

  if (loading) {
    return <LoadingSpinner />;
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full">
        <ExclamationTriangleIcon className="h-12 w-12 text-red-500 mb-4" />
        <p className="text-lg font-medium text-gray-900 dark:text-white">Error loading recordings</p>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-2">{error}</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            Recordings
          </h2>
          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Manage and playback your recorded streams
          </p>
        </div>

        {/* View Mode Selector */}
        <div className="flex items-center space-x-2">
          <button
            type="button"
            onClick={() => setViewMode('list')}
            className={`px-3 py-2 text-sm font-medium rounded-lg ${
              viewMode === 'list'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300'
            }`}
          >
            List
          </button>
          <button
            type="button"
            onClick={() => setViewMode('calendar')}
            className={`px-3 py-2 text-sm font-medium rounded-lg ${
              viewMode === 'calendar'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300'
            }`}
          >
            Calendar
          </button>
          <button
            type="button"
            onClick={() => setViewMode('timeline')}
            className={`px-3 py-2 text-sm font-medium rounded-lg ${
              viewMode === 'timeline'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300'
            }`}
          >
            Timeline
          </button>
        </div>
      </div>

      {/* Storage Overview */}
      <StorageChart
        totalSize={totalSize}
        recordings={recordings}
        streams={streams}
      />

      {/* Filters and Search */}
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
        <div className="flex flex-col sm:flex-row gap-4">
          {/* Search */}
          <div className="flex-1">
            <div className="relative">
              <MagnifyingGlassIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 h-5 w-5 text-gray-400" />
              <input
                type="text"
                placeholder="Search recordings..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-10 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
            </div>
          </div>

          {/* Stream Filter */}
          <select
            value={selectedStream}
            onChange={(e) => setSelectedStream(e.target.value)}
            className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="all">All Streams</option>
            {streams.map(stream => (
              <option key={stream.id} value={stream.id}>
                {stream.id}
              </option>
            ))}
          </select>

          {/* Sort Options */}
          <select
            value={`${sortField}-${sortOrder}`}
            onChange={(e) => {
              const [field, order] = e.target.value.split('-');
              setSortField(field as SortField);
              setSortOrder(order as SortOrder);
            }}
            className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="created_at-desc">Newest First</option>
            <option value="created_at-asc">Oldest First</option>
            <option value="size-desc">Largest First</option>
            <option value="size-asc">Smallest First</option>
            <option value="duration-desc">Longest First</option>
            <option value="duration-asc">Shortest First</option>
            <option value="filename-asc">Name (A-Z)</option>
            <option value="filename-desc">Name (Z-A)</option>
          </select>
        </div>
      </div>

      {/* Bulk Actions Toolbar */}
      {selectedRecordings.size > 0 && (
        <BulkActionsToolbar
          selectedCount={selectedRecordings.size}
          onDownload={handleBulkDownload}
          onDelete={handleBulkDelete}
          onClear={() => setSelectedRecordings(new Set())}
        />
      )}

      {/* View Content */}
      {viewMode === 'list' && (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          {filteredRecordings.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12">
              <FolderIcon className="h-12 w-12 text-gray-400 mb-4" />
              <p className="text-lg font-medium text-gray-900 dark:text-white">No recordings found</p>
              <p className="text-sm text-gray-500 dark:text-gray-400 mt-2">
                {searchQuery || selectedStream !== 'all'
                  ? 'Try adjusting your filters'
                  : 'Start recording streams to see them here'}
              </p>
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                <thead className="bg-gray-50 dark:bg-gray-900">
                  <tr>
                    <th className="px-6 py-3">
                      <input
                        type="checkbox"
                        checked={selectedRecordings.size === filteredRecordings.length}
                        onChange={selectAllRecordings}
                        className="h-4 w-4 rounded text-blue-600 focus:ring-blue-500"
                      />
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                      Recording
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                      Stream
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                      Size
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                      Duration
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                      Created
                    </th>
                    <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                      Actions
                    </th>
                  </tr>
                </thead>
                <tbody className="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                  {filteredRecordings.map(recording => (
                    <tr
                      key={recording.filename}
                      className="hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                      <td className="px-6 py-4">
                        <input
                          type="checkbox"
                          checked={selectedRecordings.has(recording.filename)}
                          onChange={() => toggleRecordingSelection(recording.filename)}
                          className="h-4 w-4 rounded text-blue-600 focus:ring-blue-500"
                        />
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <div className="flex items-center">
                          <FilmIcon className="h-5 w-5 text-gray-400 mr-2" />
                          <div>
                            <div className="text-sm font-medium text-gray-900 dark:text-white">
                              {recording.filename}
                            </div>
                          </div>
                        </div>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        {recording.stream ? (
                          <div className="flex items-center">
                            <div className={`h-2 w-2 rounded-full mr-2 ${
                              recording.stream.status === 'active' ? 'bg-green-500' :
                              recording.stream.status === 'error' ? 'bg-red-500' :
                              'bg-gray-500'
                            }`} />
                            <span className="text-sm text-gray-900 dark:text-white">
                              {recording.stream.id}
                            </span>
                          </div>
                        ) : (
                          <span className="text-sm text-gray-500 dark:text-gray-400">
                            {recording.stream_id || 'Unknown'}
                          </span>
                        )}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-white">
                        {formatSize(recording.size)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-white">
                        {formatDuration(recording.duration)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                        {formatDate(recording.created_at)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                        <div className="flex items-center justify-end space-x-2">
                          <button
                            type="button"
                            onClick={() => handlePlay(recording)}
                            className="text-blue-600 hover:text-blue-900 dark:text-blue-400 dark:hover:text-blue-300"
                            title="Play"
                          >
                            <PlayIcon className="h-5 w-5" />
                          </button>
                          <button
                            type="button"
                            onClick={() => handleDownload(recording)}
                            className="text-green-600 hover:text-green-900 dark:text-green-400 dark:hover:text-green-300"
                            title="Download"
                          >
                            <ArrowDownTrayIcon className="h-5 w-5" />
                          </button>
                          <button
                            type="button"
                            onClick={() => handleDelete(recording)}
                            className="text-red-600 hover:text-red-900 dark:text-red-400 dark:hover:text-red-300"
                            title="Delete"
                          >
                            <TrashIcon className="h-5 w-5" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}

      {viewMode === 'calendar' && (
        <CalendarView
          recordings={filteredRecordings}
          onDateSelect={(date) => {
            setDateRange({
              start: date,
              end: new Date(date.getTime() + 24 * 60 * 60 * 1000)
            });
          }}
          onRecordingClick={handlePlay}
        />
      )}

      {viewMode === 'timeline' && (
        <TimelineView
          recordings={filteredRecordings}
          onRecordingClick={handlePlay}
        />
      )}

      {/* Player Modal */}
      {playerOpen && selectedRecording && (
        <RecordingPlayer
          recording={selectedRecording}
          onClose={() => setPlayerOpen(false)}
        />
      )}

      {/* Delete Confirmation Modal */}
      {deleteModalOpen && (
        <DeleteConfirmationModal
          isOpen={deleteModalOpen}
          onClose={() => setDeleteModalOpen(false)}
          onConfirm={confirmDelete}
          recordingName={bulkDeleteMode
            ? `${selectedRecordings.size} recordings`
            : recordingToDelete?.filename || ''}
          isBulk={bulkDeleteMode}
        />
      )}
    </div>
  );
}
