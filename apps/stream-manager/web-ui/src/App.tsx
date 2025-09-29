import { useState, useEffect } from 'react';

function App() {
  const [apiStatus, setApiStatus] = useState<'checking' | 'online' | 'offline'>('checking');
  const [darkMode, setDarkMode] = useState(() => {
    return localStorage.getItem('darkMode') === 'true';
  });

  useEffect(() => {
    checkApiStatus();
    const interval = setInterval(checkApiStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (darkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
    localStorage.setItem('darkMode', darkMode.toString());
  }, [darkMode]);

  const checkApiStatus = async () => {
    try {
      const response = await fetch('/api/health', {
        method: 'GET',
        headers: { 'Accept': 'application/json' }
      });
      setApiStatus(response.ok ? 'online' : 'offline');
    } catch {
      setApiStatus('offline');
    }
  };

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 transition-colors">
      <div className="container mx-auto px-4 py-8">
        <header className="mb-8">
          <div className="flex justify-between items-center">
            <h1 className="text-3xl font-bold text-gray-900 dark:text-gray-100">
              Stream Manager
            </h1>
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <div className={`w-3 h-3 rounded-full ${
                  apiStatus === 'online' ? 'bg-green-500' :
                  apiStatus === 'offline' ? 'bg-red-500' : 'bg-yellow-500'
                }`} />
                <span className="text-sm text-gray-600 dark:text-gray-400">
                  API {apiStatus}
                </span>
              </div>
              <button
                type="button"
                onClick={() => setDarkMode(!darkMode)}
                className="p-2 rounded-lg bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                aria-label="Toggle dark mode"
              >
                {darkMode ? '☀️' : '🌙'}
              </button>
            </div>
          </div>
        </header>

        <main className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
          <div className="text-center py-12">
            <h2 className="text-2xl font-semibold text-gray-800 dark:text-gray-200 mb-4">
              Welcome to Stream Manager
            </h2>
            <p className="text-gray-600 dark:text-gray-400 mb-8">
              Multi-Stream Recording and Inference System
            </p>
            <div className="grid gap-4 md:grid-cols-3 max-w-2xl mx-auto">
              <div className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                <h3 className="font-semibold text-gray-800 dark:text-gray-200 mb-2">
                  📹 Stream Management
                </h3>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  Monitor and control multiple video streams
                </p>
              </div>
              <div className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                <h3 className="font-semibold text-gray-800 dark:text-gray-200 mb-2">
                  💾 Recording Control
                </h3>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  Start, stop, and manage recordings
                </p>
              </div>
              <div className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                <h3 className="font-semibold text-gray-800 dark:text-gray-200 mb-2">
                  📊 Real-time Metrics
                </h3>
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  View system health and performance
                </p>
              </div>
            </div>
          </div>
        </main>

        <footer className="mt-8 text-center text-sm text-gray-500 dark:text-gray-400">
          <p>Stream Manager v0.1.0 | Built with React, Vite, and Tailwind CSS</p>
        </footer>
      </div>
    </div>
  );
}

export default App;