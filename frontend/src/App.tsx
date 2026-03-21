import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import Login from './pages/Login';
import Register from './pages/Register';
import ProtectedRoute from './components/ProtectedRoute';
import { useAuth } from './context/AuthContext';

const DashboardPlaceholder = () => {
  const { user, logout } = useAuth();
  return (
    <div className="min-h-screen bg-background text-white p-8">
      <div className="max-w-4xl mx-auto bg-surface p-8 rounded-xl border border-surfaceHighlight">
        <h1 className="text-3xl font-bold text-success mb-4">Successfully logged in!</h1>
        <p className="text-gray-400 mb-6">Current user: <span className="text-primary">{user?.email}</span></p>
        <button 
          onClick={logout}
          className="px-4 py-2 bg-danger hover:bg-red-600 rounded-lg font-medium transition-colors"
        >
          Log out
        </button>
      </div>
    </div>
  );
};

function App() {
  return (
    <BrowserRouter>
      <Routes>
        {/* Public routes */}
        <Route path="/login" element={<Login />} />
        <Route path="/register" element={<Register />} />

        {/* Protected routes */}
        <Route element={<ProtectedRoute />}>
          <Route path="/dashboard" element={<DashboardPlaceholder />} />
        </Route>

        {/* Fallback route */}
        <Route path="/" element={<Navigate to="/dashboard" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;