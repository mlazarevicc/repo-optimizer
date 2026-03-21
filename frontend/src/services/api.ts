import axios from 'axios';

const api = axios.create({
  baseURL: 'http://localhost:8006/api',
  headers: {
    'Content-Type': 'application/json',
  },
});

// Interceptor
api.interceptors.request.use((config) => {
  const token = localStorage.getItem('token');
  if (token && config.headers) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

export default api;