import React, { useState, useMemo, useEffect, useCallback } from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { DiffEditor } from '@monaco-editor/react';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from 'recharts';
import {
  ArrowLeft, ShieldAlert, AlertTriangle, AlertCircle, Info,
  CheckCircle, CheckCircle2, FileCode, ChevronDown, ChevronRight,
  Loader2, Trash2, TrendingUp, Lightbulb, Filter
} from 'lucide-react';
import api from '../services/api';

// ─── Types ────────────────────────────────────────────────────────────────────

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

interface Suggestion {
  id: string;
  problem_id: string;
  explanation: string;
  original_code: string;
  suggested_code: string;
  impact_score: number;
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
  suggestions: Suggestion[];
}

// ─── Constants ────────────────────────────────────────────────────────────────

const SEVERITY_COLORS = {
  critical: '#ef4444',
  high:     '#f97316',
  medium:   '#eab308',
  low:      '#3b82f6',
};

const SEVERITY_ORDER: Record<string, number> = {
  critical: 0, high: 1, medium: 2, low: 3,
};

// Detect Monaco editor language from file extension or problem_type
const detectEditorLanguage = (issue: Issue | null): string => {
  if (!issue) return 'plaintext';
  const path = issue.file_path?.toLowerCase() ?? '';
  if (path.endsWith('.py'))   return 'python';
  if (path.endsWith('.js'))   return 'javascript';
  if (path.endsWith('.ts'))   return 'typescript';
  if (path.endsWith('.tsx'))  return 'typescript';
  if (path.endsWith('.jsx'))  return 'javascript';
  if (path.endsWith('.rs'))   return 'rust';
  if (path.endsWith('.java')) return 'java';
  if (path.endsWith('.go'))   return 'go';
  // Fallback: guess from problem_type prefix
  const pt = issue.problem_type.toLowerCase();
  if (pt.startsWith('python')) return 'python';
  if (pt.startsWith('javascript') || pt.startsWith('js')) return 'javascript';
  return 'plaintext';
};

type SeverityFilter = 'all' | 'critical' | 'high' | 'medium' | 'low';

// ─── Sub-components ───────────────────────────────────────────────────────────

const SeverityIcon = ({ severity, className = 'w-5 h-5' }: { severity: string; className?: string }) => {
  switch (severity.toLowerCase()) {
    case 'critical': return <ShieldAlert className={`${className} text-danger`} />;
    case 'high':     return <AlertTriangle className={`${className} text-warning`} />;
    case 'medium':   return <AlertCircle className={`${className} text-yellow-500`} />;
    case 'low':      return <Info className={`${className} text-info`} />;
    default:         return <Info className={className} />;
  }
};

const SeverityBadge = ({ severity }: { severity: string }) => {
  const styles: Record<string, string> = {
    critical: 'bg-danger/20 text-danger border border-danger/30',
    high:     'bg-warning/20 text-warning border border-warning/30',
    medium:   'bg-yellow-500/20 text-yellow-400 border border-yellow-500/30',
    low:      'bg-info/20 text-info border border-info/30',
  };
  return (
    <span className={`text-xs px-2.5 py-1 rounded-full font-bold uppercase tracking-wider ${styles[severity] ?? 'bg-surfaceHighlight text-muted'}`}>
      {severity}
    </span>
  );
};

const ScoreBar = ({ score, max = 1, color = '#3b82f6' }: { score: number; max?: number; color?: string }) => (
  <div className="w-full bg-surfaceHighlight rounded-full h-1.5 overflow-hidden">
    <div
      className="h-full rounded-full transition-all duration-500"
      style={{ width: `${Math.min(100, (score / max) * 100)}%`, backgroundColor: color }}
    />
  </div>
);

// ─── Main Component ───────────────────────────────────────────────────────────

const Results = () => {
  const { jobId } = useParams<{ jobId: string }>();
  const navigate = useNavigate();

  const [data, setData] = useState<AnalysisData | null>(null);
  const [loadingMsg, setLoadingMsg] = useState('Initializing analysis...');
  const [error, setError] = useState<string | null>(null);
  const [selectedIssueId, setSelectedIssueId] = useState<string | null>(null);
  const [expandedFiles, setExpandedFiles] = useState<Record<string, boolean>>({});
  const [severityFilter, setSeverityFilter] = useState<SeverityFilter>('all');
  const [isDeleting, setIsDeleting] = useState(false);

  // ── Polling loop ────────────────────────────────────────────────────────────
  useEffect(() => {
    if (!jobId) { navigate('/dashboard'); return; }

    let interval: ReturnType<typeof setInterval>;

    const fetchResults = async () => {
      try {
        const res = await api.get(`/results/${jobId}`);
        const d = res.data;
        if (d.status === 'COMPLETED') {
          setData(d);
          clearInterval(interval);
        } else if (d.status === 'FAILED') {
          setError('Analysis failed on the server. Please try again.');
          clearInterval(interval);
        } else {
          setLoadingMsg(d.message || 'Processing code...');
        }
      } catch (err: any) {
        setError(err.response?.data?.error ?? 'Failed to fetch analysis results.');
        clearInterval(interval);
      }
    };

    fetchResults();
    interval = setInterval(fetchResults, 3000);
    return () => clearInterval(interval);
  }, [jobId, navigate]);

  // ── Derived data ─────────────────────────────────────────────────────────────
  const filteredIssues = useMemo(() => {
    if (!data?.ranked_issues) return [];
    return data.ranked_issues.filter(
      issue => severityFilter === 'all' || issue.severity === severityFilter,
    );
  }, [data, severityFilter]);

  const groupedIssues = useMemo(() => {
    return filteredIssues.reduce((acc, issue) => {
      const path = issue.file_path || 'Raw Snippet';
      if (!acc[path]) acc[path] = [];
      acc[path].push(issue);
      return acc;
    }, {} as Record<string, Issue[]>);
  }, [filteredIssues]);

  const filePaths = Object.keys(groupedIssues).sort();

  // Auto-expand first file and select first issue when data arrives
  useEffect(() => {
    if (filePaths.length > 0 && Object.keys(expandedFiles).length === 0) {
      setExpandedFiles({ [filePaths[0]]: true });
      const first = groupedIssues[filePaths[0]]?.[0];
      if (first && !selectedIssueId) setSelectedIssueId(first.id);
    }
  }, [filePaths.join(',')]); // eslint-disable-line react-hooks/exhaustive-deps

  const selectedIssue = data?.ranked_issues.find(i => i.id === selectedIssueId) ?? null;
  const selectedSuggestion = data?.suggestions.find(s => s.problem_id === selectedIssueId) ?? null;
  const editorLanguage = detectEditorLanguage(selectedIssue);

  const pieData = useMemo(() => [
    { name: 'Critical', value: data?.summary.critical_count ?? 0, color: SEVERITY_COLORS.critical },
    { name: 'High',     value: data?.summary.high_count     ?? 0, color: SEVERITY_COLORS.high },
    { name: 'Medium',   value: data?.summary.medium_count   ?? 0, color: SEVERITY_COLORS.medium },
    { name: 'Low',      value: data?.summary.low_count      ?? 0, color: SEVERITY_COLORS.low },
  ].filter(d => d.value > 0), [data]);

  // ── Actions ──────────────────────────────────────────────────────────────────
  const handleDeleteJob = useCallback(async () => {
    if (!jobId) return;
    if (!window.confirm(
      'Permanently delete all analysis data for this job? This action cannot be undone (GDPR deletion).'
    )) return;
    setIsDeleting(true);
    try {
      await api.delete(`/results/${jobId}`);
      navigate('/dashboard');
    } catch {
      alert('Failed to delete job. Please try again.');
      setIsDeleting(false);
    }
  }, [jobId, navigate]);

  const toggleFile = (path: string) =>
    setExpandedFiles(prev => ({ ...prev, [path]: !prev[path] }));

  // ── Loading / error screens ──────────────────────────────────────────────────
  if (error) {
    return (
      <div className="flex flex-col h-screen items-center justify-center bg-background text-foreground">
        <ShieldAlert className="w-16 h-16 text-danger mb-4" />
        <h2 className="text-2xl font-bold mb-2">Analysis Failed</h2>
        <p className="text-muted mb-6 text-center max-w-md">{error}</p>
        <Link
          to="/dashboard"
          className="px-6 py-2 bg-primary text-primaryForeground rounded-lg font-medium hover:bg-primaryHover transition-colors"
        >
          Back to Dashboard
        </Link>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="flex flex-col h-screen items-center justify-center bg-background text-foreground gap-6">
        <div className="relative">
          <Loader2 className="w-16 h-16 text-primary animate-spin" />
        </div>
        <div className="text-center">
          <h2 className="text-2xl font-bold mb-2">Analyzing your codebase</h2>
          <p className="text-muted text-lg animate-pulse">{loadingMsg}</p>
        </div>
        <Link to="/dashboard" className="text-sm text-muted hover:text-foreground transition-colors mt-4">
          ← Back to Dashboard
        </Link>
      </div>
    );
  }

  // ── Main results layout ──────────────────────────────────────────────────────
  return (
    <div className="flex flex-col h-screen bg-background text-foreground">

      {/* Header */}
      <header className="flex items-center justify-between px-6 py-4 bg-surface border-b border-surfaceHighlight/50 shrink-0">
        <div className="flex items-center gap-4">
          <Link
            to="/dashboard"
            className="p-2 hover:bg-surfaceHighlight/50 rounded-lg transition-colors text-muted hover:text-foreground"
          >
            <ArrowLeft className="w-5 h-5" />
          </Link>
          <div>
            <h1 className="text-xl font-bold bg-gradient-to-r from-primary to-primaryHover bg-clip-text text-transparent">
              Analysis Results
            </h1>
            <p className="text-xs text-muted font-mono">Job: {data.analysis_job_id}</p>
          </div>
        </div>

        <button
          onClick={handleDeleteJob}
          disabled={isDeleting}
          className="flex items-center gap-2 px-3 py-2 text-sm text-muted hover:text-danger border border-surfaceHighlight hover:border-danger/40 rounded-lg transition-all disabled:opacity-50"
          title="Delete all data for this job (GDPR)"
        >
          {isDeleting
            ? <Loader2 className="w-4 h-4 animate-spin" />
            : <Trash2 className="w-4 h-4" />
          }
          <span className="hidden sm:inline">Delete Job</span>
        </button>
      </header>

      <div className="flex flex-1 overflow-hidden">

        {/* ── Left Sidebar ─────────────────────────────────────────────────── */}
        <div className="w-80 min-w-[300px] border-r border-surfaceHighlight/50 flex flex-col bg-surface/30 shrink-0">

          {/* Summary + Pie */}
          <div className="p-4 border-b border-surfaceHighlight/50 bg-surface/50">
            <div className="flex items-center justify-between mb-3">
              <h2 className="font-semibold text-sm text-foreground">
                {data.summary.total_problems} issue{data.summary.total_problems !== 1 ? 's' : ''} found
              </h2>
            </div>

            {pieData.length > 0 ? (
              <div className="h-36">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={pieData}
                      innerRadius={36}
                      outerRadius={60}
                      paddingAngle={2}
                      dataKey="value"
                      startAngle={90}
                      endAngle={-270}
                    >
                      {pieData.map((entry, i) => (
                        <Cell key={i} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip
                      contentStyle={{ backgroundColor: '#171717', border: '1px solid #262626', borderRadius: '8px' }}
                      itemStyle={{ color: '#f5f5f5' }}
                    />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center h-36 gap-2">
                <CheckCircle2 className="w-10 h-10 text-success" />
                <p className="text-sm text-muted">No issues detected!</p>
              </div>
            )}

            {/* Severity legend */}
            <div className="grid grid-cols-2 gap-1.5 mt-2">
              {['critical', 'high', 'medium', 'low'].map(sev => {
                const count = data.summary[`${sev}_count` as keyof typeof data.summary] as number;
                if (count === 0) return null;
                return (
                  <div key={sev} className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: SEVERITY_COLORS[sev as keyof typeof SEVERITY_COLORS] }} />
                    <span className="text-xs text-muted capitalize">{sev}</span>
                    <span className="text-xs font-mono ml-auto text-foreground">{count}</span>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Severity filter */}
          <div className="px-3 py-2 border-b border-surfaceHighlight/50 flex items-center gap-1.5 flex-wrap">
            <Filter className="w-3.5 h-3.5 text-muted shrink-0" />
            {(['all', 'critical', 'high', 'medium', 'low'] as SeverityFilter[]).map(f => (
              <button
                key={f}
                onClick={() => setSeverityFilter(f)}
                className={`text-xs px-2 py-0.5 rounded-full transition-colors capitalize ${
                  severityFilter === f
                    ? 'bg-primary text-primaryForeground'
                    : 'text-muted hover:text-foreground hover:bg-surfaceHighlight/50'
                }`}
              >
                {f}
              </button>
            ))}
          </div>

          {/* Issues list */}
          <div className="flex-1 overflow-y-auto p-2 space-y-2 custom-scrollbar">
            {filePaths.length === 0 ? (
              <div className="text-center text-muted p-6 text-sm">
                {severityFilter !== 'all'
                  ? `No ${severityFilter} issues. Try a different filter.`
                  : 'No issues found. Great job!'}
              </div>
            ) : (
              filePaths.map(path => (
                <div key={path} className="border border-surfaceHighlight/50 rounded-lg overflow-hidden bg-background">
                  {/* File header */}
                  <button
                    onClick={() => toggleFile(path)}
                    className="w-full flex items-center justify-between p-2.5 bg-surface/50 hover:bg-surfaceHighlight/30 transition-colors"
                  >
                    <div className="flex items-center gap-2 overflow-hidden">
                      {expandedFiles[path]
                        ? <ChevronDown className="w-3.5 h-3.5 text-muted shrink-0" />
                        : <ChevronRight className="w-3.5 h-3.5 text-muted shrink-0" />}
                      <FileCode className="w-3.5 h-3.5 text-primary shrink-0" />
                      <span className="font-mono text-xs truncate text-foreground" title={path}>{path}</span>
                    </div>
                    <span className="bg-surfaceHighlight px-1.5 py-0.5 rounded-full text-xs font-medium ml-2 shrink-0">
                      {groupedIssues[path].length}
                    </span>
                  </button>

                  {/* Issue rows */}
                  {expandedFiles[path] && (
                    <div className="divide-y divide-surfaceHighlight/30">
                      {groupedIssues[path]
                        .sort((a, b) => SEVERITY_ORDER[a.severity] - SEVERITY_ORDER[b.severity])
                        .map(issue => (
                          <button
                            key={issue.id}
                            onClick={() => setSelectedIssueId(issue.id)}
                            className={`w-full text-left p-3 hover:bg-surfaceHighlight/20 transition-all border-l-2 ${
                              selectedIssueId === issue.id
                                ? 'bg-primary/5 border-primary'
                                : 'border-transparent'
                            }`}
                          >
                            <div className="flex items-start gap-2.5">
                              <SeverityIcon severity={issue.severity} className="w-4 h-4 mt-0.5 shrink-0" />
                              <div className="flex-1 min-w-0">
                                <div className="flex items-center justify-between gap-1 mb-0.5">
                                  <span className="font-medium text-xs text-foreground truncate">
                                    {issue.problem_type.replace(/_/g, ' ')}
                                  </span>
                                  <span className="text-xs font-mono text-muted bg-surface px-1 rounded shrink-0">
                                    L{issue.line_start}
                                  </span>
                                </div>
                                <p className="text-xs text-muted line-clamp-2 leading-relaxed">{issue.message}</p>
                                {/* Rank score mini-bar */}
                                <div className="mt-1.5">
                                  <ScoreBar
                                    score={issue.rank_score}
                                    color={SEVERITY_COLORS[issue.severity] ?? '#3b82f6'}
                                  />
                                </div>
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

        {/* ── Right Panel ──────────────────────────────────────────────────── */}
        <div className="flex-1 flex flex-col bg-background overflow-hidden">
          {selectedIssue ? (
            <>
              {/* Issue header */}
              <div className="p-5 border-b border-surfaceHighlight/50 bg-surface shrink-0">
                <div className="flex items-center gap-2 text-xs text-muted mb-2 font-mono">
                  <FileCode className="w-3.5 h-3.5" />
                  <span className="truncate">{selectedIssue.file_path || 'Raw Snippet'}</span>
                  <span className="px-2 py-0.5 bg-surfaceHighlight rounded text-foreground shrink-0">
                    Line {selectedIssue.line_start}
                    {selectedIssue.line_end !== selectedIssue.line_start && `–${selectedIssue.line_end}`}
                  </span>
                </div>

                <div className="flex items-start gap-4">
                  <div className="bg-background p-2.5 rounded-xl border border-surfaceHighlight shadow-sm shrink-0">
                    <SeverityIcon severity={selectedIssue.severity} className="w-7 h-7" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-3 flex-wrap mb-1.5">
                      <h2 className="text-lg font-bold text-foreground">
                        {selectedIssue.problem_type.replace(/_/g, ' ')}
                      </h2>
                      <SeverityBadge severity={selectedIssue.severity} />
                    </div>
                    <p className="text-sm text-muted leading-relaxed">{selectedIssue.message}</p>

                    {/* Rank score */}
                    <div className="flex items-center gap-3 mt-3">
                      <TrendingUp className="w-3.5 h-3.5 text-muted shrink-0" />
                      <div className="flex-1">
                        <ScoreBar
                          score={selectedIssue.rank_score}
                          color={SEVERITY_COLORS[selectedIssue.severity] ?? '#3b82f6'}
                        />
                      </div>
                      <span className="text-xs font-mono text-muted shrink-0">
                        Priority {Math.round(selectedIssue.rank_score * 100)}/100
                      </span>
                    </div>
                  </div>
                </div>
              </div>

              {/* Suggestion area */}
              <div className="flex-1 overflow-hidden flex flex-col p-5 gap-4">
                {selectedSuggestion ? (
                  <>
                    {/* Explanation card */}
                    <div className="bg-surface border border-surfaceHighlight/60 rounded-xl p-4 shrink-0">
                      <div className="flex items-start gap-3">
                        <div className="bg-success/10 p-2 rounded-lg shrink-0">
                          <Lightbulb className="w-4 h-4 text-success" />
                        </div>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center justify-between mb-2">
                            <h3 className="text-sm font-semibold text-foreground">How to fix it</h3>
                            <div className="flex items-center gap-2 shrink-0">
                              <span className="text-xs text-muted">Impact:</span>
                              <span className="text-xs font-bold text-success">
                                {selectedSuggestion.impact_score}/100
                              </span>
                            </div>
                          </div>
                          <p className="text-sm text-muted leading-relaxed">
                            {selectedSuggestion.explanation}
                          </p>
                        </div>
                      </div>
                    </div>

                    {/* Diff editor */}
                    <div className="flex-1 border border-surfaceHighlight/50 rounded-xl overflow-hidden shadow-lg bg-[#1e1e1e] flex flex-col min-h-0">
                      <div className="bg-[#252526] text-gray-400 text-xs py-2 px-4 flex gap-4 border-b border-[#333] shrink-0">
                        <div className="flex-1 font-mono flex items-center gap-2">
                          <span className="text-danger">●</span> Original Code
                        </div>
                        <div className="flex-1 font-mono flex items-center gap-2">
                          <span className="text-success">●</span> Suggested Fix
                        </div>
                      </div>
                      <DiffEditor
                        height="100%"
                        language={editorLanguage}
                        theme="vs-dark"
                        original={selectedSuggestion.original_code}
                        modified={selectedSuggestion.suggested_code}
                        options={{
                          readOnly: true,
                          minimap: { enabled: false },
                          renderSideBySide: true,
                          fontSize: 13,
                          fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
                          scrollBeyondLastLine: false,
                          wordWrap: 'on',
                        }}
                      />
                    </div>
                  </>
                ) : (
                  <div className="flex-1 flex flex-col items-center justify-center text-muted gap-3">
                    <CheckCircle className="w-12 h-12 text-surfaceHighlight" />
                    <p className="text-base font-medium">No automated fix available for this issue.</p>
                    <p className="text-sm text-center max-w-sm">
                      This type of problem requires manual review. Use the description above as guidance.
                    </p>
                  </div>
                )}
              </div>
            </>
          ) : (
            /* Empty state — no issue selected */
            <div className="flex-1 flex flex-col items-center justify-center text-muted gap-3">
              <CheckCircle className="w-14 h-14 text-surfaceHighlight" />
              <p className="text-lg font-medium">Select an issue to view details</p>
              <p className="text-sm">Click any item in the left panel to see the fix suggestion.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default Results;
