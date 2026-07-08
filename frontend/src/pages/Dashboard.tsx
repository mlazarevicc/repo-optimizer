import React, { useState, useRef, useEffect, useCallback } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import Editor from '@monaco-editor/react';
import {
  Play, Code2, LogOut, Loader2, AlertCircle,
  Github, FileArchive, UploadCloud, History,
  CheckCircle2, Clock, XCircle, ChevronRight, Trash2
} from 'lucide-react';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';

const SUPPORTED_LANGUAGES = [
  { id: 'python',     name: 'Python',     defaultCode: 'import sqlite3\nimport random\n\ndef process_user(user_id, items):\n    # Hardcoded secret - security issue\n    api_key = "AKIAIOSFODNN7EXAMPLE"\n    \n    # SQL Injection vulnerability\n    conn = sqlite3.connect("db.sqlite3")\n    cursor = conn.cursor()\n    query = "SELECT * FROM users WHERE id = \'" + str(user_id) + "\'"\n    cursor.execute(query)\n    \n    # String concat in loop - performance issue\n    result = ""\n    for item in items:\n        result += str(item)\n    return result\n' },
  { id: 'javascript', name: 'JavaScript', defaultCode: 'function generateToken(userId) {\n  // Insecure random - security issue\n  const token = Math.random().toString(36);\n  \n  // String concat in loop\n  let html = "";\n  const items = [1, 2, 3, 4, 5];\n  for (const item of items) {\n    html += `<li>${item}</li>`;\n  }\n  \n  return { token, html };\n}\n' },
  { id: 'typescript', name: 'TypeScript', defaultCode: 'function calculateSum(a: number, b: number): number {\n    // TODO: Implement this function\n    return 0;\n}\n' },
  { id: 'rust',       name: 'Rust',       defaultCode: 'fn main() {\n    // TODO: Implement\n}\n' },
  { id: 'java',       name: 'Java',       defaultCode: 'public class Main {\n    public static void main(String[] args) {\n        // TODO: Implement\n    }\n}\n' },
  { id: 'go',         name: 'Go',         defaultCode: 'package main\n\nimport "fmt"\n\nfunc main() {\n    fmt.Println("Hello, RepoOptimizer!")\n}\n' },
];

type InputMode = 'code' | 'zip' | 'git';

interface Job {
  id: string;
  status: string;
  language: string;
  file_name: string | null;
  total_files: number;
  processed_files: number;
  created_at: string;
  completed_at: string | null;
}

const statusIcon = (status: string) => {
  switch (status) {
    case 'completed': return <CheckCircle2 className="w-3.5 h-3.5 text-success shrink-0" />;
    case 'processing': return <Loader2 className="w-3.5 h-3.5 text-primary animate-spin shrink-0" />;
    case 'failed': return <XCircle className="w-3.5 h-3.5 text-danger shrink-0" />;
    default: return <Clock className="w-3.5 h-3.5 text-muted shrink-0" />;
  }
};

const formatDate = (iso: string) => {
  const d = new Date(iso);
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
};

const Dashboard = () => {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  const [language, setLanguage] = useState(SUPPORTED_LANGUAGES[0]);
  const [code, setCode] = useState(SUPPORTED_LANGUAGES[0].defaultCode);
  const [error, setError] = useState<string | null>(null);
  const [isAnalyzing, setIsAnalyzing] = useState(false);

  const [inputMode, setInputMode] = useState<InputMode>('code');
  const [gitUrl, setGitUrl] = useState('');
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [showHistory, setShowHistory] = useState(false);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [jobsLoading, setJobsLoading] = useState(false);
  const [deletingJobId, setDeletingJobId] = useState<string | null>(null);

  const fileInputRef = useRef<HTMLInputElement>(null);

  const loadJobs = useCallback(async () => {
    setJobsLoading(true);
    try {
      const res = await api.get('/jobs');
      setJobs(res.data.jobs || []);
    } catch {
      // silently ignore - history is non-critical
    } finally {
      setJobsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (showHistory) loadJobs();
  }, [showHistory, loadJobs]);

  const handleAnalyze = async () => {
    setIsAnalyzing(true);
    setError(null);
    try {
      let response;
      if (inputMode === 'code') {
        response = await api.post('/analyze', { code, language: language.id });
      } else if (inputMode === 'git') {
        if (!gitUrl.trim()) throw new Error('Please enter a valid Git repository URL.');
        response = await api.post('/analyze/git', { repo_url: gitUrl });
      } else if (inputMode === 'zip') {
        if (!selectedFile) throw new Error('Please select a ZIP file to analyze.');
        const formData = new FormData();
        formData.append('file', selectedFile);
        response = await api.post('/analyze/zip', formData, {
          headers: { 'Content-Type': 'multipart/form-data' },
        });
      }
      const jobId = response?.data?.analysis_job_id;
      if (jobId) navigate(`/results/${jobId}`);
    } catch (err: any) {
      setError(err.response?.data?.error || err.message || 'An error occurred during analysis.');
    } finally {
      setIsAnalyzing(false);
    }
  };

  const handleDeleteJob = async (e: React.MouseEvent, jobId: string) => {
    e.stopPropagation();
    if (!window.confirm('Permanently delete all data for this analysis? This cannot be undone.')) return;
    setDeletingJobId(jobId);
    try {
      await api.delete(`/results/${jobId}`);
      setJobs(prev => prev.filter(j => j.id !== jobId));
    } catch {
      alert('Failed to delete job.');
    } finally {
      setDeletingJobId(null);
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      const file = e.target.files[0];
      if (!file.name.endsWith('.zip')) {
        setError('Only .zip files are supported.');
        setSelectedFile(null);
        return;
      }
      setSelectedFile(file);
      setError(null);
    }
  };

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      {/* Header */}
      <header className="flex items-center justify-between px-6 py-4 bg-surface border-b border-surfaceHighlight/50 shadow-sm shrink-0">
        <div className="flex items-center gap-3">
          <div className="bg-primary/10 p-2 rounded-lg">
            <Code2 className="w-6 h-6 text-primary" />
          </div>
          <h1 className="text-xl font-bold bg-gradient-to-r from-primary to-primaryHover bg-clip-text text-transparent">
            RepoOptimizer
          </h1>
        </div>

        <div className="flex items-center gap-3">
          {inputMode === 'code' && (
            <select
              value={language.id}
              onChange={(e) => {
                const lang = SUPPORTED_LANGUAGES.find((l) => l.id === e.target.value)!;
                setLanguage(lang);
                setCode(lang.defaultCode);
              }}
              className="bg-background border border-surfaceHighlight text-foreground text-sm rounded-lg focus:ring-primary focus:border-primary block w-40 p-2.5 transition-colors"
            >
              {SUPPORTED_LANGUAGES.map((lang) => (
                <option key={lang.id} value={lang.id}>{lang.name}</option>
              ))}
            </select>
          )}

          <button
            onClick={handleAnalyze}
            disabled={isAnalyzing}
            className="flex items-center px-4 py-2 bg-primary hover:bg-primaryHover text-primaryForeground font-medium rounded-lg transition-all active:scale-95 disabled:opacity-50 disabled:pointer-events-none shadow-md shadow-primary/20"
          >
            {isAnalyzing ? (
              <><Loader2 className="w-4 h-4 mr-2 animate-spin" />Analyzing...</>
            ) : (
              <><Play className="w-4 h-4 mr-2" />Run Analysis</>
            )}
          </button>

          <div className="w-px h-8 bg-surfaceHighlight/50" />

          <button
            onClick={() => setShowHistory(!showHistory)}
            title="Analysis History"
            className={`p-2 rounded-lg transition-colors ${showHistory ? 'text-primary bg-primary/10' : 'text-muted hover:text-foreground hover:bg-surfaceHighlight/50'}`}
          >
            <History className="w-5 h-5" />
          </button>

          <div className="flex items-center gap-2 text-sm text-muted">
            <span className="hidden sm:block">{user?.email}</span>
          </div>

          <button
            onClick={logout}
            className="p-2 text-muted hover:text-danger transition-colors rounded-lg hover:bg-danger/10"
            title="Sign Out"
          >
            <LogOut className="w-5 h-5" />
          </button>
        </div>
      </header>

      {/* Error Banner */}
      {error && (
        <div className="bg-danger/10 border-b border-danger/20 p-3 flex items-center justify-center text-danger shrink-0">
          <AlertCircle className="w-5 h-5 mr-2" />
          <span className="text-sm font-medium">{error}</span>
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        {/* Main Content */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Tab Navigation */}
          <div className="flex justify-center border-b border-surfaceHighlight/50 bg-surface/50 shrink-0">
            {(['code', 'git', 'zip'] as InputMode[]).map((mode) => {
              const labels: Record<InputMode, { icon: React.ReactNode; label: string }> = {
                code: { icon: <Code2 className="w-4 h-4 mr-2" />, label: 'Code Snippet' },
                git:  { icon: <Github className="w-4 h-4 mr-2" />, label: 'Git Repository' },
                zip:  { icon: <FileArchive className="w-4 h-4 mr-2" />, label: 'ZIP Archive' },
              };
              const { icon, label } = labels[mode];
              return (
                <button
                  key={mode}
                  onClick={() => setInputMode(mode)}
                  className={`px-6 py-3 font-medium flex items-center border-b-2 transition-colors ${
                    inputMode === mode ? 'border-primary text-primary' : 'border-transparent text-muted hover:text-foreground'
                  }`}
                >
                  {icon}{label}
                </button>
              );
            })}
          </div>

          {/* Dynamic Content */}
          <main className="flex-1 relative flex flex-col bg-background">
            {inputMode === 'code' && (
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
                    <div className="flex items-center justify-center h-full text-muted">
                      <Loader2 className="w-8 h-8 animate-spin text-primary" />
                    </div>
                  }
                />
              </div>
            )}

            {inputMode === 'git' && (
              <div className="flex-1 flex items-center justify-center p-6">
                <div className="bg-surface border border-surfaceHighlight rounded-xl p-8 max-w-2xl w-full shadow-lg">
                  <div className="flex flex-col items-center mb-6">
                    <div className="bg-primary/10 p-4 rounded-full mb-4">
                      <Github className="w-12 h-12 text-primary" />
                    </div>
                    <h2 className="text-2xl font-bold mb-2">Analyze Git Repository</h2>
                    <p className="text-muted text-center">Enter a public Git repository URL to perform a full codebase analysis.</p>
                  </div>
                  <div className="flex flex-col gap-2">
                    <label className="text-sm font-medium text-foreground">Repository URL</label>
                    <input
                      type="text"
                      value={gitUrl}
                      onChange={(e) => setGitUrl(e.target.value)}
                      placeholder="https://github.com/username/repository.git"
                      className="w-full bg-background border border-surfaceHighlight rounded-lg px-4 py-3 focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all text-foreground"
                    />
                  </div>
                </div>
              </div>
            )}

            {inputMode === 'zip' && (
              <div className="flex-1 flex items-center justify-center p-6">
                <div className="bg-surface border border-surfaceHighlight rounded-xl p-8 max-w-2xl w-full shadow-lg text-center">
                  <div className="flex flex-col items-center mb-6">
                    <div className="bg-primary/10 p-4 rounded-full mb-4">
                      <FileArchive className="w-12 h-12 text-primary" />
                    </div>
                    <h2 className="text-2xl font-bold mb-2">Upload ZIP Archive</h2>
                    <p className="text-muted">Upload a .zip file containing your source code.</p>
                  </div>
                  <div
                    onClick={() => fileInputRef.current?.click()}
                    className="border-2 border-dashed border-surfaceHighlight hover:border-primary bg-background/50 hover:bg-primary/5 rounded-xl p-12 cursor-pointer transition-all flex flex-col items-center justify-center group"
                  >
                    <input type="file" ref={fileInputRef} className="hidden" accept=".zip" onChange={handleFileChange} />
                    <UploadCloud className="w-12 h-12 text-muted group-hover:text-primary mb-4 transition-colors" />
                    {selectedFile ? (
                      <span className="text-lg font-medium text-primary">{selectedFile.name}</span>
                    ) : (
                      <>
                        <span className="text-lg font-medium mb-1">Click to browse or drag & drop</span>
                        <span className="text-sm text-muted">Maximum file size: 50MB</span>
                      </>
                    )}
                  </div>
                </div>
              </div>
            )}
          </main>
        </div>

        {/* History Sidebar */}
        {showHistory && (
          <aside className="w-80 border-l border-surfaceHighlight/50 bg-surface/30 flex flex-col shrink-0">
            <div className="p-4 border-b border-surfaceHighlight/50 flex items-center justify-between">
              <h2 className="font-semibold text-sm text-foreground">Analysis History</h2>
              <button onClick={loadJobs} className="text-muted hover:text-primary text-xs transition-colors">Refresh</button>
            </div>

            <div className="flex-1 overflow-y-auto custom-scrollbar">
              {jobsLoading ? (
                <div className="flex items-center justify-center p-8">
                  <Loader2 className="w-6 h-6 animate-spin text-primary" />
                </div>
              ) : jobs.length === 0 ? (
                <div className="p-6 text-center text-muted text-sm">
                  <History className="w-10 h-10 mx-auto mb-3 opacity-30" />
                  No analyses yet. Run your first analysis!
                </div>
              ) : (
                <div className="divide-y divide-surfaceHighlight/50">
                  {jobs.map((job) => (
                    <div
                      key={job.id}
                      className="group flex items-start gap-3 p-4 hover:bg-surfaceHighlight/20 transition-colors"
                    >
                      {/* Clickable part */}
                      <Link to={`/results/${job.id}`} className="flex-1 min-w-0 flex items-start gap-3">
                        <div className="mt-0.5">{statusIcon(job.status)}</div>
                        <div className="min-w-0">
                          <p className="text-sm font-medium text-foreground truncate">
                            {job.file_name || `${job.language} snippet`}
                          </p>
                          <p className="text-xs text-muted mt-0.5">
                            {formatDate(job.created_at)}
                          </p>
                          <div className="flex items-center gap-2 mt-1.5">
                            <span className="text-xs px-2 py-0.5 rounded-full bg-surfaceHighlight text-muted capitalize font-mono">
                              {job.language}
                            </span>
                            {job.status === 'processing' && (
                              <span className="text-xs text-muted">
                                {job.processed_files}/{job.total_files} files
                              </span>
                            )}
                          </div>
                        </div>
                      </Link>

                      {/* Delete button */}
                      {job.status === 'completed' && (
                        <button
                          onClick={(e) => handleDeleteJob(e, job.id)}
                          disabled={deletingJobId === job.id}
                          className="opacity-0 group-hover:opacity-100 p-1.5 text-muted hover:text-danger rounded transition-all"
                          title="Delete analysis data"
                        >
                          {deletingJobId === job.id
                            ? <Loader2 className="w-4 h-4 animate-spin" />
                            : <Trash2 className="w-4 h-4" />
                          }
                        </button>
                      )}

                      <Link to={`/results/${job.id}`} className="opacity-0 group-hover:opacity-100 p-1.5 text-muted hover:text-primary rounded transition-all">
                        <ChevronRight className="w-4 h-4" />
                      </Link>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </aside>
        )}
      </div>
    </div>
  );
};

export default Dashboard;
