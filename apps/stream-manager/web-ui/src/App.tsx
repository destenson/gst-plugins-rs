import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { ThemeProvider } from './contexts/ThemeContext.tsx';
import { APIProvider } from './contexts/APIContext.tsx';
import { AuthProvider } from './contexts/AuthContext.tsx';
import { WebSocketProvider } from './lib/websocket/index.ts';
import ErrorBoundary from './components/ErrorBoundary.tsx';
import ProtectedRoute from './components/ProtectedRoute.tsx';
import SessionTimeoutModal from './components/SessionTimeoutModal.tsx';
import Layout from './components/layout/Layout.tsx';
import Login from './pages/Login.tsx';
import Dashboard from './pages/Dashboard.tsx';
import Streams from './pages/Streams.tsx';
import Recordings from './pages/Recordings.tsx';
import Configuration from './pages/Configuration.tsx';
import Metrics from './pages/Metrics.tsx';
import Logs from './pages/Logs.tsx';
import Help from './pages/Help.tsx';
import Database from './pages/Database.tsx';

// Create router with future flags enabled
const router = createBrowserRouter(
  [
    {
      path: "/login",
      element: <Login />
    },
    {
      path: "/",
      element: (
        <ProtectedRoute>
          <Layout />
        </ProtectedRoute>
      ),
      children: [
        { index: true, element: <Dashboard /> },
        { path: "streams", element: <Streams /> },
        { path: "recordings", element: <Recordings /> },
        { path: "configuration", element: <Configuration /> },
        { path: "metrics", element: <Metrics /> },
        { path: "logs", element: <Logs /> },
        { path: "database", element: <Database /> },
        { path: "help", element: <Help /> },
      ]
    }
  ],
  {
    future: {
      v7_relativeSplatPath: true,
    }
  }
);

function App() {
  // Get WebSocket configuration from environment
  const wsConfig = {
    port: parseInt((window as any).VITE_API_PORT || '8080'),
    debug: (window as any).DEV || false,
  };

  // Check if development mode - always true for development
  const isDevelopmentMode = true; // Set to false for production

  return (
    <ErrorBoundary>
      <APIProvider>
        <AuthProvider developmentMode={isDevelopmentMode}>
          <WebSocketProvider config={wsConfig} autoConnect>
            <ThemeProvider>
              <RouterProvider router={router} />
              <SessionTimeoutModal />
            </ThemeProvider>
          </WebSocketProvider>
        </AuthProvider>
      </APIProvider>
    </ErrorBoundary>
  );
}

export default App;