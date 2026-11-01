import { Component, type ErrorInfo, type ReactNode } from "react";
import { AlertTriangle, RotateCcw } from "lucide-react";

export class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Valotrack page error", error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="page">
          <div className="board-empty error-boundary">
            <div className="empty-icon">
              <AlertTriangle size={20} />
            </div>
            <h2>This page hit a snag</h2>
            <p>{this.state.error.message}</p>
            <button
              className="button primary"
              onClick={() => {
                this.setState({ error: null });
                location.reload();
              }}
            >
              <RotateCcw size={14} /> Reload the app
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
