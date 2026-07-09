import { Component, type ReactNode, type ErrorInfo, type CSSProperties } from 'react';

interface ErrorBoundaryProps {
  name: string;
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

const containerStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 'var(--gap-md)',
  padding: 'var(--gap-2xl)',
  textAlign: 'center',
  minHeight: 200,
};

const titleStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '12px',
  fontWeight: 500,
  letterSpacing: '0.06em',
  textTransform: 'uppercase' as const,
  color: 'var(--rose-bright)',
};

const messageStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '12px',
  color: 'var(--text-dim)',
  maxWidth: 480,
};

const buttonStyle: CSSProperties = {
  fontFamily: 'var(--mono)',
  fontSize: '11px',
  fontWeight: 500,
  letterSpacing: '0.06em',
  textTransform: 'uppercase' as const,
  color: 'var(--text-primary)',
  padding: '8px 20px',
  border: '1px solid var(--border)',
  background: 'var(--bg-glass)',
  cursor: 'pointer',
  transition: `border-color var(--duration-fast) var(--ease-out), background-color var(--duration-fast) var(--ease-out)`,
};

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(`[ErrorBoundary:${this.props.name}]`, error, info.componentStack);
  }

  handleReload = () => {
    this.setState({ error: null });
  };

  render() {
    if (this.state.error) {
      return (
        <div style={containerStyle}>
          <div style={titleStyle}>Error in {this.props.name}</div>
          <div style={messageStyle}>
            {this.state.error.message}
          </div>
          <button
            style={buttonStyle}
            onClick={this.handleReload}
            onMouseEnter={e => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-strong)'; }}
            onMouseLeave={e => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border)'; }}
          >
            Reload section
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
