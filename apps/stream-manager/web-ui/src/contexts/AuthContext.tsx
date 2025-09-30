import React, {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useState,
} from "react";
import { useAPI } from "./APIContext.tsx";

interface User {
  id: string;
  username: string;
  email?: string;
  role?: string;
}

interface AuthContextValue {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (username: string, password: string, rememberMe?: boolean) => Promise<void>;
  logout: () => Promise<void>;
  checkAuth: () => Promise<void>;
  sessionTimeoutWarning: boolean;
  extendSession: () => void;
  isDevelopmentMode: boolean;
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined);

const SESSION_TIMEOUT = 30 * 60 * 1000; // 30 minutes
const SESSION_WARNING_TIME = 5 * 60 * 1000; // 5 minutes before expiry

interface AuthProviderProps {
  children: ReactNode;
  developmentMode?: boolean;
}

export const AuthProvider: React.FC<AuthProviderProps> = ({
  children,
  developmentMode = (globalThis as any).DEV || false,
}) => {
  const { api, setToken } = useAPI();
  const [user, setUser] = useState<User | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [sessionTimeoutWarning, setSessionTimeoutWarning] = useState(false);
  const [sessionTimer, setSessionTimer] = useState<number | null>(null);
  const [warningTimer, setWarningTimer] = useState<number | null>(null);

  // Clear all timers
  const clearTimers = useCallback(() => {
    if (sessionTimer) {
      globalThis.clearTimeout(sessionTimer);
      setSessionTimer(null);
    }
    if (warningTimer) {
      globalThis.clearTimeout(warningTimer);
      setWarningTimer(null);
    }
  }, [sessionTimer, warningTimer]);

  // Set up session timers
  const setupSessionTimers = useCallback(() => {
    clearTimers();

    // Warning timer - shows warning 5 minutes before expiry
    const warning = globalThis.setTimeout(() => {
      setSessionTimeoutWarning(true);
    }, SESSION_TIMEOUT - SESSION_WARNING_TIME);
    setWarningTimer(warning as unknown as number);

    // Session expiry timer
    const session = globalThis.setTimeout(() => {
      logout();
    }, SESSION_TIMEOUT);
    setSessionTimer(session as unknown as number);
  }, [clearTimers]);

  // Extend session
  const extendSession = useCallback(() => {
    setSessionTimeoutWarning(false);
    setupSessionTimers();
  }, [setupSessionTimers]);

  // Check authentication status on mount
  const checkAuth = useCallback(async () => {
    setIsLoading(true);

    // Development mode bypass
    if (developmentMode) {
      setUser({
        id: "dev-user",
        username: "developer",
        email: "dev@example.com",
        role: "admin",
      });
      setIsAuthenticated(true);
      setIsLoading(false);
      return;
    }

    try {
      // Check for stored token
      const storedToken = localStorage.getItem("api_token");
      const rememberMe = localStorage.getItem("remember_me") === "true";

      if (!storedToken) {
        // Check session storage for non-persistent sessions
        const sessionToken = sessionStorage.getItem("api_token");
        if (sessionToken) {
          setToken(sessionToken);
          // Verify token with API
          const userInfo = await api.auth.verify();
          setUser(userInfo);
          setIsAuthenticated(true);
          setupSessionTimers();
        }
      } else if (rememberMe) {
        setToken(storedToken);
        // Verify token with API
        const userInfo = await api.auth.verify();
        setUser(userInfo);
        setIsAuthenticated(true);
        setupSessionTimers();
      }
    } catch (error) {
      console.error("Auth check failed:", error);
      setUser(null);
      setIsAuthenticated(false);
      setToken(null);
    } finally {
      setIsLoading(false);
    }
  }, [developmentMode, api, setToken, setupSessionTimers]);

  // Login function
  const login = useCallback(async (username: string, password: string, rememberMe = false) => {
    setIsLoading(true);

    // Development mode bypass
    if (developmentMode && username === "dev" && password === "dev") {
      const devUser = {
        id: "dev-user",
        username: "developer",
        email: "dev@example.com",
        role: "admin",
      };
      setUser(devUser);
      setIsAuthenticated(true);
      setIsLoading(false);

      // Store token for dev mode
      const devToken = "dev-token-" + Date.now();
      if (rememberMe) {
        localStorage.setItem("api_token", devToken);
        localStorage.setItem("remember_me", "true");
      } else {
        sessionStorage.setItem("api_token", devToken);
        localStorage.removeItem("remember_me");
      }
      setToken(devToken);
      setupSessionTimers();
      return;
    }

    try {
      const response = await api.auth.login({ username, password });
      const { token, user: userInfo } = response;

      setUser(userInfo);
      setIsAuthenticated(true);
      setToken(token);

      // Store token based on remember me preference
      if (rememberMe) {
        localStorage.setItem("api_token", token);
        localStorage.setItem("remember_me", "true");
      } else {
        sessionStorage.setItem("api_token", token);
        localStorage.removeItem("remember_me");
        localStorage.removeItem("api_token");
      }

      setupSessionTimers();
    } catch (error: any) {
      console.error("Login failed:", error);
      throw new Error(error?.message || "Invalid credentials");
    } finally {
      setIsLoading(false);
    }
  }, [developmentMode, api, setToken, setupSessionTimers]);

  // Logout function
  const logout = useCallback(async () => {
    try {
      // Call logout API if authenticated
      if (isAuthenticated && !developmentMode) {
        await api.auth.logout();
      }
    } catch (error) {
      console.error("Logout API call failed:", error);
    } finally {
      // Clear all auth state
      setUser(null);
      setIsAuthenticated(false);
      setToken(null);

      // Clear storage
      localStorage.removeItem("api_token");
      localStorage.removeItem("remember_me");
      sessionStorage.removeItem("api_token");

      // Clear timers
      clearTimers();
      setSessionTimeoutWarning(false);

      // Trigger auth:logout event
      globalThis.dispatchEvent(new Event("auth:logout"));
    }
  }, [isAuthenticated, developmentMode, api, setToken, clearTimers]);

  // Check auth on mount
  useEffect(() => {
    checkAuth();
  }, []);

  // Clean up timers on unmount
  useEffect(() => {
    return () => {
      clearTimers();
    };
  }, [clearTimers]);

  const value: AuthContextValue = {
    user,
    isAuthenticated,
    isLoading,
    login,
    logout,
    checkAuth,
    sessionTimeoutWarning,
    extendSession,
    isDevelopmentMode: developmentMode,
  };

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
};

// Custom hook to use the auth context
export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within AuthProvider");
  }
  return context;
};

export default AuthContext;
