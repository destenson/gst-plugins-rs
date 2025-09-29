import { Bars3Icon, MoonIcon, SunIcon } from '@heroicons/react/24/outline';
import { useTheme } from '../../contexts/ThemeContext.tsx';
import { ConnectionStatus } from '../../lib/websocket/index.ts';
import Breadcrumb from '../Breadcrumb.tsx';

interface HeaderProps {
  setSidebarOpen: (open: boolean) => void;
}

export default function Header({ setSidebarOpen }: HeaderProps) {
  const { darkMode, toggleDarkMode } = useTheme();

  return (
    <div className="sticky top-0 z-40 lg:mx-auto lg:px-8">
      <div className="flex h-16 items-center gap-x-4 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 px-4 shadow-sm sm:gap-x-6 sm:px-6 lg:px-0 lg:shadow-none">
        <button
          type="button"
          className="-m-2.5 p-2.5 text-gray-700 dark:text-gray-200 lg:hidden"
          onClick={() => setSidebarOpen(true)}
        >
          <span className="sr-only">Open sidebar</span>
          <Bars3Icon className="h-6 w-6" aria-hidden="true" />
        </button>

        {/* Separator */}
        <div className="h-6 w-px bg-gray-200 dark:bg-gray-700 lg:hidden" aria-hidden="true" />

        <div className="flex flex-1 gap-x-4 self-stretch lg:gap-x-6">
          <div className="flex flex-1 items-center">
            <Breadcrumb />
          </div>

          <div className="flex items-center gap-x-4 lg:gap-x-6">
            {/* WebSocket connection status */}
            <ConnectionStatus compact />

            {/* Theme toggle */}
            <button
              type="button"
              onClick={toggleDarkMode}
              className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200"
              aria-label={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
            >
              {darkMode ? (
                <SunIcon className="h-5 w-5" aria-hidden="true" />
              ) : (
                <MoonIcon className="h-5 w-5" aria-hidden="true" />
              )}
            </button>

            {/* Profile dropdown placeholder */}
            <div className="hidden lg:block lg:h-6 lg:w-px lg:bg-gray-200 dark:bg-gray-700" aria-hidden="true" />

            {/* User menu placeholder */}
            <div className="flex items-center">
              <button
                type="button"
                className="flex items-center gap-x-2 p-2 text-sm font-medium text-gray-700 dark:text-gray-200 hover:text-gray-900 dark:hover:text-white"
              >
                <span className="inline-block h-8 w-8 rounded-full bg-gray-300 dark:bg-gray-600" />
                <span className="hidden lg:block">User</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}