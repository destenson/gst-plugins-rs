import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { ThemeProvider } from './contexts/ThemeContext.tsx';
import { APIProvider } from './contexts/APIContext.tsx';
import { WebSocketProvider } from './lib/websocket/index.ts';
import ErrorBoundary from './components/ErrorBoundary.tsx';
import Layout from './components/layout/Layout.tsx';
import Dashboard from './pages/Dashboard.tsx';
import Streams from './pages/Streams.tsx';
import Recordings from './pages/Recordings.tsx';
import Configuration from './pages/Configuration.tsx';
import Metrics from './pages/Metrics.tsx';
import Logs from './pages/Logs.tsx';
import Help from './pages/Help.tsx';

function App() {
  // Get WebSocket configuration from environment
  const wsConfig = {
    port: parseInt((window as any).VITE_API_PORT || '8080'),
    debug: (window as any).DEV || false,
  };

  return (
    <ErrorBoundary>
      <APIProvider>
        <WebSocketProvider config={wsConfig} autoConnect>
          <ThemeProvider>
            <BrowserRouter>
              <Routes>
                <Route path="/" element={<Layout />}>
                  <Route index element={<Dashboard />} />
                  <Route path="streams" element={<Streams />} />
                  <Route path="recordings" element={<Recordings />} />
                  <Route path="configuration" element={<Configuration />} />
                  <Route path="metrics" element={<Metrics />} />
                  <Route path="logs" element={<Logs />} />
                  <Route path="help" element={<Help />} />
                  <Route path="*" element={<NotFound />} />
                </Route>
              </Routes>
            </BrowserRouter>
          </ThemeProvider>
        </WebSocketProvider>
      </APIProvider>
    </ErrorBoundary>
  );
}

function NotFound() {
  return (
    <div className="flex flex-col items-center justify-center h-64">
      <h1 className="text-4xl font-bold text-gray-900 dark:text-white mb-4">404</h1>
      <p className="text-gray-600 dark:text-gray-400">Page not found</p>
    </div>
  );
}

export default App;