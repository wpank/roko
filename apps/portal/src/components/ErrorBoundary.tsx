'use client';
import React from 'react';

interface ErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
  /** Name of the section for error reporting */
  name?: string;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error(`[ErrorBoundary${this.props.name ? `: ${this.props.name}` : ''}]`, error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;
      return (
        <div className="p-4 border border-[var(--accent-error)] bg-[var(--bg-raised)]">
          <h3 className="font-mono text-sm text-[var(--accent-error)] mb-2">
            {this.props.name ? `${this.props.name}: ` : ''}Something went wrong
          </h3>
          <pre className="font-mono text-xs text-[var(--text-muted)] whitespace-pre-wrap">
            {this.state.error?.message}
          </pre>
          <button
            onClick={() => this.setState({ hasError: false, error: null })}
            className="mt-2 font-mono text-xs text-[var(--rose)] hover:text-[var(--rose-bright)] underline"
          >
            Try again
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
