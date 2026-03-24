import React, { useState } from 'react';
import { useLocation, Navigate, Link } from 'react-router-dom';
import { DiffEditor } from '@monaco-editor/react';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip } from 'recharts';
import { ArrowLeft, ShieldAlert, AlertTriangle, AlertCircle, Info, CheckCircle } from 'lucide-react';

interface AnalysisData {
  analysis_job_id: string;
  summary: {
    critical_count: number;
    high_count: number;
    medium_count: number;
    low_count: number;
    total_problems: number;
  };
  ranked_issues: any[];
  suggestions: any[];
}

const COLORS = {
  critical: '#ef4444', // red-500
  high: '#f97316',     // orange-500
  medium: '#eab308',   // yellow-500
  low: '#3b82f6'       // blue-500
};

const Results = () => {
  const location = useLocation();
  const data = location.state?.analysisData as AnalysisData;

  if (!data) {
    return <Navigate to="/dashboard" replace />;
  }

  // State to track which problem is currently selected in the list
  const [selectedIssueId, setSelectedIssueId] = useState<string | null>(
    data.ranked_issues.length > 0 ? data.ranked_issues[0].id : null
  );

  const selectedIssue = data.ranked_issues.find(issue => issue.id === selectedIssueId);
  const selectedSuggestion = data.suggestions.find(sug => sug.problem_id === selectedIssueId);

  const chartData = [
    { name: 'Critical', value: data.summary.critical_count, color: COLORS.critical },
    { name: 'High', value: data.summary.high_count, color: COLORS.high },
    { name: 'Medium', value: data.summary.medium_count, color: COLORS.medium },
    { name: 'Low', value: data.summary.low_count, color: COLORS.low },
  ].filter(item => item.value > 0);

  const getSeverityIcon = (severity: string) => {
    switch (severity.toLowerCase()) {
      case 'critical': return <ShieldAlert className="w-5 h-5 text-danger" />;
      case 'high': return <AlertTriangle className="w-5 h-5 text-orange-500" />;
      case 'medium': return <AlertCircle className="w-5 h-5 text-warning" />;
      case 'low': return <Info className="w-5 h-5 text-primary" />;
      default: return <Info className="w-5 h-5 text-gray-400" />;
    }
  };

  const formatProblemTitle = (rawType: string) => {
    const parts = rawType.split('.');
    const lastPart = parts[parts.length - 1];
    return lastPart.replace(/[-_]/g, ' ').toUpperCase();
  };

  return (
    <div className="min-h-screen bg-background flex flex-col text-gray-100">
      {/* Header */}
      <header className="h-16 border-b border-surfaceHighlight bg-surface flex items-center px-6 shrink-0">
        <Link to="/dashboard" className="flex items-center text-gray-400 hover:text-white transition-colors">
          <ArrowLeft className="w-5 h-5 mr-2" />
          Back to Editor
        </Link>
        <div className="mx-auto flex items-center">
          <h1 className="text-xl font-bold">Analysis Report</h1>
          <span className="ml-3 px-3 py-1 bg-surfaceHighlight text-xs rounded-full text-gray-400 font-mono">
            {data.analysis_job_id.split('-')[0]}
          </span>
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        
        {/* Left Sidebar: Issues List & Chart */}
        <div className="w-1/3 border-r border-surfaceHighlight bg-surface flex flex-col overflow-y-auto">
          {/* Chart Section */}
          <div className="p-6 border-b border-surfaceHighlight">
            <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">Severity Breakdown</h2>
            {data.summary.total_problems === 0 ? (
              <div className="flex flex-col items-center justify-center py-8">
                <CheckCircle className="w-16 h-16 text-success mb-4" />
                <p className="text-lg font-medium text-success">Perfect Code!</p>
                <p className="text-sm text-gray-400">No issues detected.</p>
              </div>
            ) : (
              <div className="h-48">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={chartData}
                      innerRadius={60}
                      outerRadius={80}
                      paddingAngle={5}
                      dataKey="value"
                    >
                      {chartData.map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={entry.color} stroke="rgba(0,0,0,0)" />
                      ))}
                    </Pie>
                    <Tooltip 
                      contentStyle={{ backgroundColor: '#171717', borderColor: '#262626', borderRadius: '8px' }}
                      itemStyle={{ color: '#f3f4f6' }}
                    />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            )}
          </div>

          {/* Issues List */}
          <div className="flex-1 p-4">
            <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4 px-2">
              Ranked Issues ({data.ranked_issues.length})
            </h2>
            <div className="space-y-2">
              {data.ranked_issues.map((issue) => (
                <button
                  key={issue.id}
                  onClick={() => setSelectedIssueId(issue.id)}
                  className={`w-full text-left p-4 rounded-xl border transition-all ${
                    selectedIssueId === issue.id 
                      ? 'bg-surfaceHighlight border-gray-500 shadow-lg' 
                      : 'bg-background border-transparent hover:border-surfaceHighlight'
                  }`}
                >
                  <div className="flex items-start justify-between">
                    <div className="flex items-center space-x-3">
                      {getSeverityIcon(issue.severity)}
                      <div>
                        <p className="font-medium text-sm text-gray-200 truncate pr-4">
                          {formatProblemTitle(issue.problem_type)}
                        </p>
                        <p className="text-xs text-gray-500 mt-1">Line {issue.line_start}</p>
                      </div>
                    </div>
                    <div className="flex flex-col items-end">
                      <span className="text-xs font-mono text-primary bg-primary/10 px-2 py-1 rounded">
                        Score: {(issue.rank_score * 100).toFixed(0)}
                      </span>
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Right Panel: Details & Diff Editor */}
        <div className="flex-1 flex flex-col bg-background relative">
          {selectedIssue ? (
            <>
              {/* Issue Details Header */}
              <div className="p-8 border-b border-surfaceHighlight shrink-0">
                <div className="flex items-center space-x-3 mb-4">
                  {getSeverityIcon(selectedIssue.severity)}
                  <h2 className="text-2xl font-bold text-gray-100">{selectedIssue.message}</h2>
                </div>
                
                {selectedSuggestion && (
                  <div className="mt-6 bg-surface p-6 rounded-xl border border-surfaceHighlight">
                    <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-2">AI Suggestion</h3>
                    <p className="text-gray-200 text-lg">{selectedSuggestion.explanation}</p>
                    <div className="mt-4 flex items-center">
                      <span className="text-sm text-gray-400 mr-2">Impact Score:</span>
                      <div className="px-3 py-1 rounded-full bg-success/10 text-success font-bold text-sm">
                        +{selectedSuggestion.impact_score}
                      </div>
                    </div>
                  </div>
                )}
              </div>

              {/* Monaco Diff Editor */}
              <div className="flex-1 relative">
                {selectedSuggestion ? (
                  <div className="absolute inset-0 pt-4">
                    <div className="flex justify-between px-8 mb-2 text-sm font-semibold text-gray-500 uppercase">
                      <span>Original Code</span>
                      <span>Suggested Fix</span>
                    </div>
                    <DiffEditor
                      height="calc(100% - 30px)"
                      language="python" // TODO: dinamički jezik iz stanja ako ga budemo prosleđivali
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