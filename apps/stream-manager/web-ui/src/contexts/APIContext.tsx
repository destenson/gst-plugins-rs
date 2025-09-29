import React, { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { StreamManagerAPI } from '../api/index.ts';
import { createMockAPI } from '../api/mock.ts';

interface APIContextValue {
  api: StreamManagerAPI | ReturnType<typeof createMockAPI>;
  isAuthenticated: boolean;
  setToken: (token: string | null) => void;
  useMockAPI: boolean;
  toggleMockAPI: () => void;
}

const APIContext = createContext<APIContextValue | undefined>(undefined);

interface APIProviderProps {
  children: ReactNode;
  baseURL?: string;
  initialToken?: string;
  forceMock?: boolean;
}

export const APIProvider: React.FC<APIProviderProps> = ({
  children,
  baseURL = (globalThis as any).VITE_API_URL || 'http://localhost:3000',
  initialToken,
  forceMock = false
}) => {
  const [token, setTokenState] = useState<string | null>(initialToken || null);
  const [isAuthenticated, setIsAuthenticated] = useState(!!initialToken);
  const [useMockAPI, setUseMockAPI] = useState(forceMock || (globalThis as any).DEV || false);

  // Create API instance
  const [api] = useState(() => {
    if (useMockAPI) {
      return createMockAPI();
    }
    return new StreamManagerAPI({ baseURL, token: initialToken });
  });

  useEffect(() => {
    // Check for stored token on mount
    const storedToken = localStorage.getItem('api_token');
    if (storedToken && !initialToken) {
      setTokenState(storedToken);
      setIsAuthenticated(true);
      if (!useMockAPI && api instanceof StreamManagerAPI) {
        api.setToken(storedToken);
      }
    }

    // Listen for auth failures
    const handleAuthFailed = () => {
      setTokenState(null);
      setIsAuthenticated(false);
      // Optionally redirect to login
      // window.location.href = '/login';
    };

    window.addEventListener('auth:failed', handleAuthFailed);
    return () => {
      window.removeEventListener('auth:failed', handleAuthFailed);
    };
  }, [api, initialToken, useMockAPI]);

  const setToken = (newToken: string | null) => {
    setTokenState(newToken);
    setIsAuthenticated(!!newToken);

    if (!useMockAPI && api instanceof StreamManagerAPI) {
      api.setToken(newToken);
    }

    if (newToken) {
      localStorage.setItem('api_token', newToken);
    } else {
      localStorage.removeItem('api_token');
    }
  };

  const toggleMockAPI = () => {
    if (!forceMock) {
      setUseMockAPI((prev: boolean) => !prev);
      // Note: This won't change the existing API instance
      // You would need to reload the app to switch between real and mock
      window.location.reload();
    }
  };

  const value: APIContextValue = {
    api,
    isAuthenticated,
    setToken,
    useMockAPI,
    toggleMockAPI
  };

  return <APIContext.Provider value={value}>{children}</APIContext.Provider>;
};

// Custom hook to use the API context
export const useAPI = () => {
  const context = useContext(APIContext);
  if (!context) {
    throw new Error('useAPI must be used within APIProvider');
  }
  return context;
};

// Export for convenience
export default APIContext;