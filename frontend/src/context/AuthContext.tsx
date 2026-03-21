import React, { createContext, useContext, useState, useEffect, type ReactNode } from 'react';

interface User {
  email: string;
  user_id: string;
}

interface AuthContextType {
  user: User | null;
  token: string | null;
  login: (token: string, email: string, user_id: string) => void;
  logout: () => void;
  isAuthenticated: boolean;
  isInitializing: boolean;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider = ({ children }: { children: ReactNode }) => {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isInitializing, setIsInitializing] = useState(true);

  useEffect(() => {
    const storedToken = localStorage.getItem('token');
    const storedEmail = localStorage.getItem('email');
    const storedUserId = localStorage.getItem('user_id');

    if (storedToken && storedEmail && storedUserId) {
      setToken(storedToken);
      setUser({ email: storedEmail, user_id: storedUserId });
    }
    
    setIsInitializing(false);
  }, []);

  const login = (newToken: string, email: string, user_id: string) => {
    localStorage.setItem('token', newToken);
    localStorage.setItem('email', email);
    localStorage.setItem('user_id', user_id);
    setToken(newToken);
    setUser({ email, user_id });
  };

  const logout = () => {
    localStorage.removeItem('token');
    localStorage.removeItem('email');
    localStorage.removeItem('user_id');
    setToken(null);
    setUser(null);
  };

  return (
    <AuthContext.Provider value={{ user, token, login, logout, isAuthenticated: !!token, isInitializing }}>
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};