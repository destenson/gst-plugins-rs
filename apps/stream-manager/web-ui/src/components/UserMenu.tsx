import React, { useEffect, useRef, useState } from "react";
import { useAuth } from "../contexts/AuthContext.tsx";
import { ChevronDown, LogOut, Settings, User } from "lucide-react";
import { Link } from "react-router-dom";

const UserMenu: React.FC = () => {
  const { user, logout, isDevelopmentMode } = useAuth();
  const [isOpen, setIsOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }

    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen]);

  // Close menu on escape key
  useEffect(() => {
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener("keydown", handleEscape);
    }

    return () => {
      document.removeEventListener("keydown", handleEscape);
    };
  }, [isOpen]);

  if (!user) {
    return null;
  }

  const handleLogout = async () => {
    setIsOpen(false);
    await logout();
  };

  return (
    <div className="relative" ref={menuRef}>
      {/* User Menu Button */}
      <button
        type="button"
        className="flex items-center space-x-3 px-3 py-2 rounded-md text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700 focus:outline-none focus:bg-gray-100 dark:focus:bg-gray-700"
        onClick={() => setIsOpen(!isOpen)}
        aria-expanded={isOpen}
        aria-haspopup="true"
      >
        <div className="flex-shrink-0 w-8 h-8 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center">
          <User className="w-4 h-4 text-blue-600 dark:text-blue-400" />
        </div>
        <div className="hidden md:block text-left">
          <div className="text-sm font-medium text-gray-900 dark:text-white">
            {user.username}
          </div>
          {user.email && (
            <div className="text-xs text-gray-500 dark:text-gray-400">
              {user.email}
            </div>
          )}
        </div>
        <ChevronDown className={`w-4 h-4 transition-transform ${isOpen ? "rotate-180" : ""}`} />
      </button>

      {/* Dropdown Menu */}
      {isOpen && (
        <div className="absolute right-0 mt-2 w-56 rounded-md shadow-lg bg-white dark:bg-gray-800 ring-1 ring-black ring-opacity-5 z-50">
          <div className="py-1" role="menu" aria-orientation="vertical">
            {/* User Info */}
            <div className="px-4 py-2 border-b border-gray-200 dark:border-gray-700">
              <p className="text-sm font-medium text-gray-900 dark:text-white">
                {user.username}
              </p>
              {user.email && (
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  {user.email}
                </p>
              )}
              {user.role && (
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  Role: {user.role}
                </p>
              )}
              {isDevelopmentMode && (
                <p className="text-xs text-yellow-600 dark:text-yellow-400 mt-1">
                  Development Mode
                </p>
              )}
            </div>

            {/* Menu Items */}
            <Link
              to="/configuration"
              className="block px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
              role="menuitem"
              onClick={() => setIsOpen(false)}
            >
              <Settings className="inline-block w-4 h-4 mr-2" />
              Settings
            </Link>

            {/* Logout */}
            <button
              type="button"
              className="block w-full text-left px-4 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700"
              role="menuitem"
              onClick={handleLogout}
            >
              <LogOut className="inline-block w-4 h-4 mr-2" />
              Sign Out
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default UserMenu;
