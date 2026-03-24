import React, { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import Editor from '@monaco-editor/react';
import { Play, Code2, LogOut, Loader2, AlertCircle } from 'lucide-react';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';

const SUPPORTED_LANGUAGES = [
  { id: 'python', name: 'Python', defaultCode: 'def calculate_sum(a, b):\n    # TODO: Implement this function\n    pass\n' },
  { id: 'javascript', name: 'JavaScript', defaultCode: 'function calculateSum(a, b) {\n    // TODO: Implement this function\n}\n' },
  { id: 'typescript', name: 'TypeScript', defaultCode: 'function calculateSum(a: number, b: number): number {\n    // TODO: Implement this function\n    return 0;\n}\n' },
  { id: 'rust', name: 'Rust', defaultCode: 'fn calculate_sum(a: i32, b: i32) -> i32 {\n    // TODO: Implement this function\n    0\n}\n' },
  { id: 'java', name: 'Java', defaultCode: 'public class Main {\n    public static void main(String[] args) {\n        // TODO: Implement this function\n    }\n}\n' },
];

const Dashboard = () => {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  
  const [language, setLanguage] = useState(SUPPORTED_LANGUAGES[0]);
  const [code, setCode] = useState(language.defaultCode);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [pollingMessage, setPollingMessage] = useState('Analyzing...');
  const [error, setError] = useState('');

  const isMounted = useRef(true);

  useEffect(() => {
    isMounted.current = true;
    return () => {
      isMounted.current = false;
    };
  }, []);

  const pollResults = async (jobId: string) => {
    if (!isMounted.current) return;

    try {
      const response = await api.get(`/results/${jobId}`);
      const data = response.data;

      if (data.status === 'COMPLETED') {
        setIsAnalyzing(false);
        navigate('/results', { state: { analysisData: data } });
      } else {
        setPollingMessage(data.message || 'Processing in background...');
        
        setTimeout(() => pollResults(jobId), 2000);
      }
    } catch (err: any) {
      if (isMounted.current) {
        setIsAnalyzing(false);
        setError('Failed to fetch analysis results. ' + (err.response?.data?.error || ''));
      }
    }
  };

  const handleAnalyze = async () => {
    if (!code.trim()) {
      setError('Please enter some code to analyze.');
      return;
    }

    setIsAnalyzing(true);
    setError('');
    setPollingMessage('Submitting code...');

    try {
      const response = await api.post('/analyze', {
        language: language.id,
        code: code,
      });

      const jobId = response.data.analysis_job_id;
      pollResults(jobId);

    } catch (err: any) {
      setIsAnalyzing(false);
      setError(err.response?.data?.error || 'Failed to submit analysis job.');
    }
  };

  const handleLanguageChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const selected = SUPPORTED_LANGUAGES.find(l => l.id === e.target.value) || SUPPORTED_LANGUAGES[0];
    setLanguage(selected);
    setCode(selected.defaultCode);
    setError('');
  };

  return (
    <div className="min-h-screen bg-background flex flex-col">
      {/* Top Navbar */}
      <header className="h-16 border-b border-surfaceHighlight bg-surface flex items-center justify-between px-6 shrink-0">
        <div className="flex items-center space-x-3">
          <div className="p-2 bg-primary/10 rounded-lg">
            <Code2 className="w-6 h-6 text-primary" />
          </div>
          <h1 className="text-xl font-bold text-gray-100">RepoOptimizer</h1>
        </div>

        <div className="flex items-center space-x-6">
          <div className="flex items-center space-x-2">
            <span className="text-sm text-gray-400">Language:</span>
            <select 
              value={language.id}
              onChange={handleLanguageChange}
              disabled={isAnalyzing}
              className="bg-background border border-surfaceHighlight text-gray-200 text-sm rounded-lg focus:ring-primary focus:border-primary block p-2 outline-none"
            >
              {SUPPORTED_LANGUAGES.map(lang => (
                <option key={lang.id} value={lang.id}>{lang.name}</option>
              ))}
            </select>
          </div>

          <button
            onClick={handleAnalyze}
            disabled={isAnalyzing}
            className="flex items-center px-4 py-2 bg-primary hover:bg-blue-600 text-white rounded-lg font-medium transition-colors disabled:opacity-50 min-w-[160px] justify-center"
          >
            {isAnalyzing ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin shrink-0" />
                <span className="truncate">{pollingMessage}</span>
              </>
            ) : (
              <>
                <Play className="w-4 h-4 mr-2" />
                Run Analysis
              </>
            )}
          </button>

          <div className="h-8 w-px bg-surfaceHighlight"></div>

          <div className="flex items-center space-x-4">
            <span className="text-sm text-gray-400">{user?.email}</span>
            <button 
              onClick={logout}
              className="p-2 text-gray-400 hover:text-danger transition-colors rounded-lg hover:bg-danger/10"
              title="Sign Out"
            >
              <LogOut className="w-5 h-5" />
            </button>
          </div>
        </div>
      </header>

      {/* Error Banner */}
      {error && (
        <div className="bg-danger/10 border-b border-danger/20 p-3 flex items-center justify-center text-danger shrink-0">
          <AlertCircle className="w-5 h-5 mr-2" />
          <span className="text-sm font-medium">{error}</span>
        </div>
      )}

      {/* Editor Area */}
      <main className="flex-1 relative">
        <div className="absolute inset-0">
          <Editor
            height="100%"
            language={language.id}
            theme="vs-dark"
            value={code}
            onChange={(value) => setCode(value || '')}
            options={{
              minimap: { enabled: false },
              fontSize: 14,
              fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
              wordWrap: 'on',
              padding: { top: 16 },
              scrollBeyondLastLine: false,
              smoothScrolling: true,
              cursorBlinking: 'smooth',
            }}
            loading={
              <div className="flex items-center justify-center h-full text-gray-400">
                <Loader2 className="w-8 h-8 animate-spin text-primary" />
              </div>
            }
          />
        </div>
      </main>
    </div>
  );
};

export default Dashboard;