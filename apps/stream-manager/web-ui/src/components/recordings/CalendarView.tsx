import React, { useMemo, useState } from "react";
import { ChevronLeftIcon, ChevronRightIcon, FilmIcon } from "@heroicons/react/24/outline";

interface CalendarViewProps {
  recordings: Array<{
    filename: string;
    created_at: string;
    duration: number;
    size: number;
    stream_id?: string;
  }>;
  onDateSelect: (date: Date) => void;
  onRecordingClick: (recording: any) => void;
}

export default function CalendarView(
  { recordings, onDateSelect, onRecordingClick }: CalendarViewProps,
) {
  const [currentMonth, setCurrentMonth] = useState(new Date());
  const [selectedDate, setSelectedDate] = useState<Date | null>(null);

  // Group recordings by date
  const recordingsByDate = useMemo(() => {
    const grouped = new Map<string, typeof recordings>();

    recordings.forEach((recording) => {
      const date = new Date(recording.created_at);
      const dateKey = `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;

      if (!grouped.has(dateKey)) {
        grouped.set(dateKey, []);
      }
      grouped.get(dateKey)!.push(recording);
    });

    return grouped;
  }, [recordings]);

  // Get calendar days for current month
  const calendarDays = useMemo(() => {
    const year = currentMonth.getFullYear();
    const month = currentMonth.getMonth();
    const firstDay = new Date(year, month, 1);
    const lastDay = new Date(year, month + 1, 0);
    const startDate = new Date(firstDay);
    startDate.setDate(startDate.getDate() - firstDay.getDay());

    const days: Date[] = [];
    const current = new Date(startDate);

    while (current <= lastDay || current.getDay() !== 0) {
      days.push(new Date(current));
      current.setDate(current.getDate() + 1);
    }

    return days;
  }, [currentMonth]);

  const handlePrevMonth = () => {
    setCurrentMonth((prev) => {
      const newMonth = new Date(prev);
      newMonth.setMonth(newMonth.getMonth() - 1);
      return newMonth;
    });
  };

  const handleNextMonth = () => {
    setCurrentMonth((prev) => {
      const newMonth = new Date(prev);
      newMonth.setMonth(newMonth.getMonth() + 1);
      return newMonth;
    });
  };

  const handleDateClick = (date: Date) => {
    setSelectedDate(date);
    onDateSelect(date);
  };

  const getRecordingsForDate = (date: Date) => {
    const dateKey = `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`;
    return recordingsByDate.get(dateKey) || [];
  };

  const isToday = (date: Date) => {
    const today = new Date();
    return date.getDate() === today.getDate() &&
      date.getMonth() === today.getMonth() &&
      date.getFullYear() === today.getFullYear();
  };

  const isSelectedDate = (date: Date) => {
    if (!selectedDate) return false;
    return date.getDate() === selectedDate.getDate() &&
      date.getMonth() === selectedDate.getMonth() &&
      date.getFullYear() === selectedDate.getFullYear();
  };

  const isCurrentMonth = (date: Date) => {
    return date.getMonth() === currentMonth.getMonth();
  };

  const formatMonth = (date: Date) => {
    return date.toLocaleDateString("en-US", { month: "long", year: "numeric" });
  };

  const formatTime = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit" });
  };

  const formatSize = (bytes: number) => {
    const units = ["B", "KB", "MB", "GB"];
    let size = bytes;
    let unitIndex = 0;
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }
    return `${size.toFixed(1)} ${units[unitIndex]}`;
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow">
      {/* Calendar Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
        <button
          onClick={handlePrevMonth}
          className="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
        >
          <ChevronLeftIcon className="h-5 w-5 text-gray-600 dark:text-gray-400" />
        </button>

        <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
          {formatMonth(currentMonth)}
        </h3>

        <button
          onClick={handleNextMonth}
          className="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
        >
          <ChevronRightIcon className="h-5 w-5 text-gray-600 dark:text-gray-400" />
        </button>
      </div>

      <div className="flex">
        {/* Calendar Grid */}
        <div className="flex-1 p-4">
          {/* Day Headers */}
          <div className="grid grid-cols-7 gap-1 mb-2">
            {["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map((day) => (
              <div
                key={day}
                className="text-center text-xs font-medium text-gray-500 dark:text-gray-400 py-2"
              >
                {day}
              </div>
            ))}
          </div>

          {/* Calendar Days */}
          <div className="grid grid-cols-7 gap-1">
            {calendarDays.map((date, index) => {
              const dayRecordings = getRecordingsForDate(date);
              const hasRecordings = dayRecordings.length > 0;

              return (
                <button
                  key={index}
                  onClick={() => handleDateClick(date)}
                  className={`
                    relative min-h-[80px] p-2 border rounded-lg transition-all
                    ${
                    isCurrentMonth(date)
                      ? "bg-white dark:bg-gray-800"
                      : "bg-gray-50 dark:bg-gray-900 opacity-50"
                  }
                    ${
                    isToday(date)
                      ? "border-blue-500 dark:border-blue-400"
                      : "border-gray-200 dark:border-gray-700"
                  }
                    ${isSelectedDate(date) ? "ring-2 ring-blue-500 dark:ring-blue-400" : ""}
                    ${
                    hasRecordings
                      ? "hover:bg-blue-50 dark:hover:bg-blue-900/20"
                      : "hover:bg-gray-50 dark:hover:bg-gray-700"
                  }
                  `}
                >
                  <div className="flex flex-col items-start w-full">
                    <span
                      className={`text-sm font-medium ${
                        isToday(date)
                          ? "text-blue-600 dark:text-blue-400"
                          : "text-gray-900 dark:text-white"
                      }`}
                    >
                      {date.getDate()}
                    </span>

                    {hasRecordings && (
                      <div className="mt-1 w-full">
                        <div className="flex items-center justify-center">
                          <FilmIcon className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                          <span className="ml-1 text-xs font-medium text-blue-600 dark:text-blue-400">
                            {dayRecordings.length}
                          </span>
                        </div>
                        <div className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                          {formatSize(dayRecordings.reduce((sum, r) => sum + r.size, 0))}
                        </div>
                      </div>
                    )}
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Selected Date Details */}
        {selectedDate && (
          <div className="w-80 border-l border-gray-200 dark:border-gray-700 p-4">
            <h4 className="text-sm font-semibold text-gray-900 dark:text-white mb-3">
              {selectedDate.toLocaleDateString("en-US", {
                weekday: "long",
                year: "numeric",
                month: "long",
                day: "numeric",
              })}
            </h4>

            <div className="space-y-2 max-h-[500px] overflow-y-auto">
              {getRecordingsForDate(selectedDate).length === 0
                ? (
                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    No recordings for this date
                  </p>
                )
                : (
                  getRecordingsForDate(selectedDate).map((recording) => (
                    <button
                      key={recording.filename}
                      onClick={() => onRecordingClick(recording)}
                      className="w-full p-3 bg-gray-50 dark:bg-gray-900 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors text-left"
                    >
                      <div className="flex items-start justify-between">
                        <div className="flex-1">
                          <div className="flex items-center">
                            <FilmIcon className="h-4 w-4 text-gray-400 mr-2" />
                            <span className="text-sm font-medium text-gray-900 dark:text-white truncate">
                              {recording.filename}
                            </span>
                          </div>
                          {recording.stream_id && (
                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                              Stream: {recording.stream_id}
                            </p>
                          )}
                          <div className="flex items-center mt-1 text-xs text-gray-500 dark:text-gray-400">
                            <span>{formatTime(recording.created_at)}</span>
                            <span className="mx-2">•</span>
                            <span>{formatSize(recording.size)}</span>
                          </div>
                        </div>
                      </div>
                    </button>
                  ))
                )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
