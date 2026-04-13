import React, { useState, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import Editor from '@monaco-editor/react';
import { Play, Code2, LogOut, Loader2, AlertCircle, Github, FileArchive, UploadCloud } from 'lucide-react';
import { useAuth } from '../context/AuthContext';
import api from '../services/api';

const SUPPORTED_LANGUAGES = [
  { id: 'python', name: 'Python', defaultCode: 'def calculate_sum(a, b):\n    # TODO: Implement this function\n    pass\n' },
  { id: 'javascript', name: 'JavaScript', defaultCode: 'function calculateSum(a, b) {\n    // TODO: Implement this function\n}\n' },
  { id: 'typescript', name: 'TypeScript', defaultCode: 'function calculateSum(a: number, b: number): number {\n    // TODO: Implement this function\n    return 0;\n}\n' },
  { id: 'rust', name: 'Rust', defaultCode: 'fn calculate_sum(a: i32, b: i32) -> i32 {\n    // TODO: Implement this function\n    0\n}\n' },
  { id: 'java', name: 'Java', defaultCode: 'public class Main {\n    public static void main(String[] args) {\n        // TODO: Implement this function\n    }\n}\n' },
];

type InputMode = 'code' | 'zip' | 'git';

const Dashboard = () => {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  
  const [language, setLanguage] = useState(SUPPORTED_LANGUAGES[0]);
  const [code, setCode] = useState(language.defaultCode);
  const [error, setError] = useState<string | null>(null);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  
  const [inputMode, setInputMode] = useState<InputMode>('code');
  const [gitUrl, setGitUrl] = useState('');
  const [selectedFile, setSelectedFile] = useState<File | null>(null);

  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleAnalyze = async () => {
    setIsAnalyzing(true);
    setError(null);

    try {
      let response;

      if (inputMode === 'code') {
        response = await api.post('/analyze', { 
          code, 
          language: language.id 
        });
      } else if (inputMode === 'git') {
        if (!gitUrl.trim()) throw new Error('Please enter a valid Git repository URL.');
        response = await api.post('/analyze/git', { 
          repo_url: gitUrl 
        });
      } else if (inputMode === 'zip') {
        if (!selectedFile) throw new Error('Please select a ZIP file to analyze.');
        const formData = new FormData();
        formData.append('file', selectedFile);
        
        response = await api.post('/analyze/zip', formData, {
          headers: { 'Content-Type': 'multipart/form-data' }
        });
      }

      const jobId = response?.data?.analysis_job_id;
      if (jobId) {
        navigate(`/results/${jobId}`); 
      }

    } catch (err: any) {
      setError(err.response?.data?.error || err.message || 'An error occurred during analysis.');
    } finally {
      setIsAnalyzing(false);
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

        <div className="flex items-center gap-6">
          {/* Language dropdown menu - only visible in 'code' mode */}
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
                <option key={lang.id} value={lang.id}>
                  {lang.name}
                </option>
              ))}
            </select>
          )}

          <button
            onClick={handleAnalyze}
            disabled={isAnalyzing}
            className="flex items-center px-4 py-2 bg-primary hover:bg-primaryHover text-primaryForeground font-medium rounded-lg transition-all active:scale-95 disabled:opacity-50 disabled:pointer-events-none shadow-md shadow-primary/20"
          >
            {isAnalyzing ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                Analyzing...
              </>
            ) : (
              <>
                <Play className="w-4 h-4 mr-2" />
                Run Analysis
              </>
            )}
          </button>

          <div className="w-px h-8 bg-surfaceHighlight/50"></div>
          
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

      {/* Tab Navigation */}
      <div className="flex justify-center border-b border-surfaceHighlight/50 bg-surface/50">
        <button 
          onClick={() => setInputMode('code')}
          className={`px-6 py-3 font-medium flex items-center border-b-2 transition-colors ${inputMode === 'code' ? 'border-primary text-primary' : 'border-transparent text-muted hover:text-foreground'}`}
        >
          <Code2 className="w-4 h-4 mr-2" /> Code Snippet
        </button>
        <button 
          onClick={() => setInputMode('git')}
          className={`px-6 py-3 font-medium flex items-center border-b-2 transition-colors ${inputMode === 'git' ? 'border-primary text-primary' : 'border-transparent text-muted hover:text-foreground'}`}
        >
          <Github className="w-4 h-4 mr-2" /> Git Repository
        </button>
        <button 
          onClick={() => setInputMode('zip')}
          className={`px-6 py-3 font-medium flex items-center border-b-2 transition-colors ${inputMode === 'zip' ? 'border-primary text-primary' : 'border-transparent text-muted hover:text-foreground'}`}
        >
          <FileArchive className="w-4 h-4 mr-2" /> ZIP Archive
        </button>
      </div>

      {/* Dynamic Content Display */}
      <main className="flex-1 relative flex flex-col bg-background">
        
        {/* MODE 1: RAW CODE */}
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
                <div className="flex items-center justify-center h-full text-gray-400">
                  <Loader2 className="w-8 h-8 animate-spin text-primary" />
                </div>
              }
            />
          </div>
        )}

        {/* MOD 2: GIT REPOSITORY */}
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
                  className="w-full bg-background border border-surfaceHighlight rounded-lg px-4 py-3 focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all"
                />
              </div>
            </div>
          </div>
        )}

        {/* MOD 3: ZIP ARCHIVE */}
        {inputMode === 'zip' && (
          <div className="flex-1 flex items-center justify-center p-6">
            <div className="bg-surface border border-surfaceHighlight rounded-xl p-8 max-w-2xl w-full shadow-lg text-center">
              <div className="flex flex-col items-center mb-6">
                <div className="bg-primary/10 p-4 rounded-full mb-4">
                  <FileArchive className="w-12 h-12 text-primary" />
                </div>
                <h2 className="text-2xl font-bold mb-2">Upload ZIP Archive</h2>
                <p className="text-muted">Upload a .zip file containing your source code. We will extract and analyze it.</p>
              </div>
              
              <div 
                onClick={() => fileInputRef.current?.click()}
                className="border-2 border-dashed border-surfaceHighlight hover:border-primary bg-background/50 hover:bg-primary/5 rounded-xl p-12 cursor-pointer transition-all flex flex-col items-center justify-center group"
              >
                <input 
                  type="file" 
                  ref={fileInputRef} 
                  className="hidden" 
                  accept=".zip"
                  onChange={handleFileChange}
                />
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
  );
};

export default Dashboard;