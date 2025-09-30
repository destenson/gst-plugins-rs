import React from "react";
import { TrashIcon, XMarkIcon } from "@heroicons/react/24/outline";
import { ArrowDownTrayIcon } from "@heroicons/react/24/solid";

interface BulkActionsToolbarProps {
  selectedCount: number;
  onDownload: () => void;
  onDelete: () => void;
  onClear: () => void;
}

export default function BulkActionsToolbar({
  selectedCount,
  onDownload,
  onDelete,
  onClear,
}: BulkActionsToolbarProps) {
  return (
    <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-4">
          <span className="text-sm font-medium text-blue-900 dark:text-blue-100">
            {selectedCount} recording{selectedCount !== 1 ? "s" : ""} selected
          </span>

          <div className="flex items-center space-x-2">
            <button
              type="button"
              onClick={onDownload}
              className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-white bg-green-600 rounded-lg hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"
              title="Download selected recordings"
            >
              <ArrowDownTrayIcon className="h-4 w-4 mr-1.5" />
              Download
            </button>

            <button
              type="button"
              onClick={onDelete}
              className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-white bg-red-600 rounded-lg hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500"
              title="Delete selected recordings"
            >
              <TrashIcon className="h-4 w-4 mr-1.5" />
              Delete
            </button>
          </div>
        </div>

        <button
          type="button"
          onClick={onClear}
          className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
          title="Clear selection"
        >
          <XMarkIcon className="h-4 w-4 mr-1.5" />
          Clear Selection
        </button>
      </div>

      <div className="mt-2 text-xs text-blue-700 dark:text-blue-300">
        Tip: Use Shift+Click to select a range of recordings
      </div>
    </div>
  );
}
