import React, { useState, useMemo, useEffect } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { DiffEditor } from '@monaco-editor/react';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from 'recharts';
import { ArrowLeft, ShieldAlert, AlertTriangle, AlertCircle, Info, CheckCircle, FileCode, ChevronDown, ChevronRight, Loader2 } from 'lucide-react';
import api from '../services/api';

interface Issue {
  id: string;
  problem_type: string;
  severity: 'critical' | 'high' | 'medium' | 'low';
  line_start: number;
  line_end: number;
  message: string;
  code_snippet: string;
  rank_score: number;
  file_path?: string;
}

interface AnalysisData {
  analysis_job_id: string;
  status: string;
  summary: {
    critical_count: number;
    high_count: number;
    medium_count: number;
    low_count: number;
    total_problems: number;
  };
  ranked_issues: Issue[];
  suggestions: any[];
}

const COLORS = {
  critical: '#ef4444',
  high: '#f97316',
  medium: '#eab308',
  low: '#3b82f6'
};

const Results = () => {
  const { jobId } = useParams<{ jobId: string }>();
  const navigate = useNavigate();
  
  const [data, setData] = useState<AnalysisData | null>(null);
  const [loadingMsg, setLoadingMsg] = useState<string>('Initializing analysis...');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!jobId) {
      navigate('/dashboard');
      return;
    }

    let interval: ReturnType<typeof setInterval>;

    const fetchResults = async () => {
      try {
        const response = await api.get(`/results/${jobId}`);
        const resultData = response.data;

        if (resultData.status === 'COMPLETED') {
          setData(resultData);
          clearInterval(interval);
        } else {
          setLoadingMsg(resultData.message || 'Processing code...');
        }
      } catch (err: any) {
        console.error("Error fetching results:", err);
        setError(err.response?.data?.error || "Failed to fetch analysis results.");
        clearInterval(interval);
      }
    };

    fetchResults();
    
    interval = setInterval(fetchResults, 3000);

    return () => clearInterval(interval);
  }, [jobId, navigate]);

  const groupedIssues = useMemo(() => {
    if (!data?.ranked_issues) return {};
    return data.ranked_issues.reduce((acc, issue) => {
      const path = issue.file_path || 'Raw Snippet';
      if (!acc[path]) acc[path] = [];
      acc[path].push(issue);
      return acc;
    }, {} as Record<string, Issue[]>);
  }, [data]);

  const filePaths = Object.keys(groupedIssues);

  const [expandedFiles, setExpandedFiles] = useState<Record<string, boolean>>({});
  const [selectedIssueId, setSelectedIssueId] = useState<string | null>(null);

  // Automatsko selektovanje prvog fajla i problema kada podaci stignu
  useEffect(() => {
    if (filePaths.length > 0 && Object.keys(expandedFiles).length === 0) {
      const firstPath = filePaths[0];
      setExpandedFiles({ [firstPath]: true });
      if (groupedIssues[firstPath]?.length > 0 && !selectedIssueId) {
        setSelectedIssueId(groupedIssues[firstPath][0].id);
      }
    }
  }, [filePaths, expandedFiles, groupedIssues, selectedIssueId]);

  const toggleFile = (path: string) => {
    setExpandedFiles(prev => ({ ...prev, [path]: !prev[path] }));
  };

  // Ekran za učitavanje
  if (error) {
    return (
      <div className="flex flex-col h-screen items-center justify-center bg-background text-foreground">
        <ShieldAlert className="w-16 h-16 text-danger mb-4" />
        <h2 className="text-2xl font-bold mb-2">Analysis Failed</h2>
        <p className="text-muted mb-6">{error}</p>
        <Link to="/dashboard" className="px-6 py-2 bg-primary text-primaryForeground rounded-lg font-medium hover:bg-primaryHover transition-colors">
          Go Back
        </Link>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="flex flex-col h-screen items-center justify-center bg-background text-foreground">
        <Loader2 className="w-16 h-16 text-primary animate-spin mb-6" />
        <h2 className="text-2xl font-bold mb-2">Analyzing your codebase</h2>
        <p className="text-muted text-lg animate-pulse">{loadingMsg}</p>
      </div>
    );
  }

  const selectedIssue = data.ranked_issues.find(issue => issue.id === selectedIssueId);
  const selectedSuggestion = data.suggestions.find(sugg => sugg.problem_id === selectedIssueId);

  const pieData = [
    { name: 'Critical', value: data.summary.critical_count, color: COLORS.critical },
    { name: 'High', value: data.summary.high_count, color: COLORS.high },
    { name: 'Medium', value: data.summary.medium_count, color: COLORS.medium },
    { name: 'Low', value: data.summary.low_count, color: COLORS.low },
  ].filter(item => item.value > 0);

  const getSeverityIcon = (severity: string, className = "w-5 h-5") => {
    switch (severity.toLowerCase()) {
      case 'critical': return <ShieldAlert className={`${className} text-danger`} />;
      case 'high': return <AlertTriangle className={`${className} text-warning`} />;
      case 'medium': return <AlertCircle className={`${className} text-yellow-500`} />;
      case 'low': return <Info className={`${className} text-info`} />;
      default: return <Info className={className} />;
    }
  };

  return (
    <div className="flex flex-col h-screen bg-background text-foreground">
      {/* Header */}
      <header className="flex items-center justify-between px-6 py-4 bg-surface border-b border-surfaceHighlight/50 shrink-0">
        <div className="flex items-center gap-4">
          <Link to="/dashboard" className="p-2 hover:bg-surfaceHighlight/50 rounded-lg transition-colors text-muted hover:text-foreground">
            <ArrowLeft className="w-5 h-5" />
          </Link>
          <div>
            <h1 className="text-xl font-bold bg-gradient-to-r from-primary to-primaryHover bg-clip-text text-transparent">
              Analysis Results
            </h1>
            <p className="text-sm text-muted">Job ID: <span className="font-mono text-xs">{data.analysis_job_id}</span></p>
          </div>
        </div>
      </header>

      <div className="flex flex-1 overflow-hidden">
        
        {/* Left Sidebar - Grouped Issues List */}
        <div className="w-1/3 min-w-[350px] border-r border-surfaceHighlight/50 flex flex-col bg-surface/30">
          <div className="p-4 border-b border-surfaceHighlight/50 bg-surface/50">
            <h2 className="font-semibold text-lg mb-4">Issues Found ({data.summary.total_problems})</h2>
            <div className="h-40">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={pieData}
                    innerRadius={40}
                    outerRadius={70}
                    paddingAngle={2}
                    dataKey="value"
                  >
                    {pieData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                  <Tooltip 
                    contentStyle={{ backgroundColor: '#1e1e24', border: '1px solid #2d2d36', borderRadius: '8px' }}
                    itemStyle={{ color: '#fff' }}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>
          </div>
          
          <div className="flex-1 overflow-y-auto p-3 space-y-4 custom-scrollbar">
            {filePaths.length === 0 ? (
              <div className="text-center text-muted p-4">No issues found. Great job!</div>
            ) : (
              filePaths.map(path => (
                <div key={path} className="border border-surfaceHighlight/50 rounded-lg overflow-hidden bg-background">
                  <button 
                    onClick={() => toggleFile(path)}
                    className="w-full flex items-center justify-between p-3 bg-surface/50 hover:bg-surfaceHighlight/30 transition-colors"
                  >
                    <div className="flex items-center gap-2 overflow-hidden">
                      {expandedFiles[path] ? <ChevronDown className="w-4 h-4 text-muted shrink-0" /> : <ChevronRight className="w-4 h-4 text-muted shrink-0" />}
                      <FileCode className="w-4 h-4 text-primary shrink-0" />
                      <span className="font-mono text-sm truncate" title={path}>{path}</span>
                    </div>
                    <span className="bg-surfaceHighlight px-2 py-0.5 rounded-full text-xs font-medium">
                      {groupedIssues[path].length}
                    </span>
                  </button>

                  {expandedFiles[path] && (
                    <div className="divide-y divide-surfaceHighlight/50">
                      {groupedIssues[path].map((issue) => (
                        <button
                          key={issue.id}
                          onClick={() => setSelectedIssueId(issue.id)}
                          className={`w-full text-left p-4 hover:bg-surfaceHighlight/20 transition-all ${
                            selectedIssueId === issue.id ? 'bg-primary/5 border-l-2 border-primary' : 'border-l-2 border-transparent'
                          }`}
                        >
                          <div className="flex items-start gap-3">
                            <div className="mt-1 shrink-0">
                              {getSeverityIcon(issue.severity)}
                            </div>
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center justify-between mb-1">
                                <span className="font-medium text-sm text-foreground truncate">{issue.problem_type}</span>
                                <span className="text-xs font-mono text-muted bg-surface px-1.5 py-0.5 rounded">
                                  L{issue.line_start}-{issue.line_end}
                                </span>
                              </div>
                              <p className="text-xs text-muted line-clamp-2">{issue.message}</p>
                            </div>
                          </div>
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              ))
            )}
          </div>
        </div>

        {/* Right Content - Issue Details */}
        <div className="flex-1 flex flex-col bg-background relative">
          {selectedIssue ? (
            <>
              <div className="p-6 border-b border-surfaceHighlight/50 bg-surface shrink-0">
                <div className="flex items-center gap-2 text-sm text-muted mb-3 font-mono">
                  <FileCode className="w-4 h-4" />
                  {selectedIssue.file_path || 'Raw Snippet'} 
                  <span className="px-2 py-0.5 bg-surfaceHighlight rounded text-foreground">Line {selectedIssue.line_start}</span>
                </div>
                <div className="flex items-start gap-4">
                  <div className="bg-background p-3 rounded-xl border border-surfaceHighlight shadow-sm">
                    {getSeverityIcon(selectedIssue.severity, "w-8 h-8")}
                  </div>
                  <div>
                    <h2 className="text-2xl font-bold mb-2 flex items-center gap-3">
                      {selectedIssue.problem_type}
                      <span className={`text-xs px-2.5 py-1 rounded-full font-bold uppercase tracking-wider ${
                        selectedIssue.severity === 'critical' ? 'bg-danger/20 text-danger' :
                        selectedIssue.severity === 'high' ? 'bg-warning/20 text-warning' :
                        selectedIssue.severity === 'medium' ? 'bg-yellow-500/20 text-yellow-500' :
                        'bg-info/20 text-info'
                      }`}>
                        {selectedIssue.severity}
                      </span>
                    </h2>
                    <p className="text-muted leading-relaxed max-w-3xl">{selectedIssue.message}</p>
                  </div>
                </div>
              </div>

              <div className="flex-1 p-6 overflow-hidden flex flex-col">
                <h3 className="text-lg font-semibold mb-4 flex items-center gap-2">
                  <CheckCircle className="w-5 h-5 text-success" />
                  Suggested Fix
                </h3>
                
                {selectedSuggestion ? (
                  <div className="flex-1 border border-surfaceHighlight/50 rounded-xl overflow-hidden shadow-lg bg-[#1e1e1e] flex flex-col">
                    <div className="bg-[#252526] text-gray-300 text-xs py-2 px-4 flex gap-4 border-b border-[#333]">
                      <div className="flex-1 font-mono">Original Code</div>
                      <div className="flex-1 font-mono text-success">Optimized Code</div>
                    </div>
                    <DiffEditor
                      height="calc(100% - 30px)"
                      language="javascript" 
                      theme="vs-dark"
                      original={selectedSuggestion.original_code}
                      modified={selectedSuggestion.suggested_code}
                      options={{
                        readOnly: true,
                        minimap: { enabled: false },
                        renderSideBySide: true,
                        fontSize: 14,
                        fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
                        scrollBeyondLastLine: false,
                      }}
                    />
                  </div>
                ) : (
                  <div className="absolute inset-0 flex items-center justify-center">
                    <p className="text-gray-500 text-lg">No automated suggestion available for this issue.</p>
                  </div>
                )}
              </div>
            </>
          ) : (
            <div className="absolute inset-0 flex flex-col items-center justify-center text-gray-500">
              <CheckCircle className="w-16 h-16 mb-4 text-surfaceHighlight" />
              <p className="text-xl font-medium">Select an issue from the list to view details.</p>
            </div>
          )}
        </div>

      </div>
    </div>
  );
};

export default Results;