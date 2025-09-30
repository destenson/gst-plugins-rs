import React, { createContext, ReactNode, useContext, useEffect, useState } from "react";
import { StreamManagerAPI } from "../api/index.ts";

interface APIContextValue {
  api: StreamManagerAPI;
  isAuthenticated: boolean;
  setToken: (token: string | null) => void;
}

const APIContext = createContext<APIContextValue | undefined>(undefined);

interface APIProviderProps {
  children: ReactNode;
  baseURL?: string;
  initialToken?: string;
}

export const APIProvider: React.FC<APIProviderProps> = ({
  children,
  baseURL = (globalThis as any).VITE_API_URL || "",
  initialToken,
}) => {
  const [, setTokenState] = useState<string | null>(initialToken || null);
  const [isAuthenticated, setIsAuthenticated] = useState(!!initialToken);

  // Create API instance
  const [api] = useState(() => {
    return new StreamManagerAPI({ baseURL, token: initialToken });
  });

  useEffect(() => {
    // Check for stored token on mount
    const storedToken = localStorage.getItem("api_token");
    if (storedToken && !initialToken) {
      setTokenState(storedToken);
      setIsAuthenticated(true);
      api.setToken(storedToken);
    }
  }, [api, initialToken]);

  const setToken = (newToken: string | null) => {
    setTokenState(newToken);
    setIsAuthenticated(!!newToken);
    api.setToken(newToken);

    if (newToken) {
      localStorage.setItem("api_token", newToken);
    } else {
      localStorage.removeItem("api_token");
    }
  };

  return (
    <APIContext.Provider
      value={{
        api,
        isAuthenticated,
        setToken,
      }}
    >
      {children}
    </APIContext.Provider>
  );
};

export const useAPI = (): APIContextValue => {
  const context = useContext(APIContext);
  if (!context) {
    throw new Error("useAPI must be used within an APIProvider");
  }
  return context;
};
