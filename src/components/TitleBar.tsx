import { getCurrentWindow } from '@tauri-apps/api/window';
import { useState, useEffect } from 'react';

interface TitleBarProps {
  onMenuClick?: () => void;
}

export default function TitleBar({ onMenuClick }: TitleBarProps) {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    const updateIsMaximized = async () => {
      const maximized = await getCurrentWindow().isMaximized();
      setIsMaximized(maximized);
    };
    
    updateIsMaximized();
    
    // Listen for resize events to update the maximize icon
    let unlisten: (() => void) | undefined;
    
    getCurrentWindow().onResized(() => {
      updateIsMaximized();
    }).then(fn => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const handleMinimize = () => {
    getCurrentWindow().minimize();
  };

  const handleMaximize = () => {
    getCurrentWindow().toggleMaximize();
  };

  const handleClose = () => {
    getCurrentWindow().close();
  };

  return (
    <div 
      className="h-10 shrink-0 flex items-center justify-between bg-surface border-b border-border select-none pl-4"
      data-tauri-drag-region
    >
      <div className="flex items-center gap-3" data-tauri-drag-region>
        {/* Mobile hamburger menu */}
        <button 
          onClick={onMenuClick}
          className="md:hidden p-1 -ml-2 rounded-md hover:bg-surface-hover text-foreground-muted hover:text-foreground transition-colors"
          aria-label="Toggle menu"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="3" y1="12" x2="21" y2="12"></line>
            <line x1="3" y1="6" x2="21" y2="6"></line>
            <line x1="3" y1="18" x2="21" y2="18"></line>
          </svg>
        </button>
        
        {/* App Title */}
        <div className="flex items-center gap-2 pointer-events-none">
          <div
            className="w-4 h-4 rounded-[0.25rem]"
            style={{
              background: "linear-gradient(135deg, var(--accent-mid), var(--accent-end))",
            }}
          />
          <span className="font-semibold text-[0.8125rem] tracking-[-0.01em] text-foreground">
            Relay
          </span>
        </div>
      </div>

      {/* Spacer to allow dragging in the middle */}
      <div className="flex-1 h-full" data-tauri-drag-region />

      {/* Window Controls */}
      <div className="flex h-full">
        <button
          onClick={handleMinimize}
          className="w-11 h-full flex items-center justify-center hover:bg-surface-hover text-foreground-muted hover:text-foreground transition-colors"
          aria-label="Minimize"
        >
          <svg width="11" height="1" viewBox="0 0 11 1" fill="currentColor">
            <rect width="11" height="1" rx="0.5" />
          </svg>
        </button>
        <button
          onClick={handleMaximize}
          className="w-11 h-full flex items-center justify-center hover:bg-surface-hover text-foreground-muted hover:text-foreground transition-colors"
          aria-label="Maximize"
        >
          {isMaximized ? (
            <svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="currentColor">
              <path d="M2.5 2.5H8.5V8.5H2.5V2.5Z" />
              <path d="M4.5 2.5V1.5H10.5V7.5H8.5" />
            </svg>
          ) : (
            <svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="currentColor">
              <rect x="1.5" y="1.5" width="8" height="8" rx="0.5" />
            </svg>
          )}
        </button>
        <button
          onClick={handleClose}
          className="w-11 h-full flex items-center justify-center hover:bg-destructive hover:text-destructive-foreground text-foreground-muted transition-colors"
          aria-label="Close"
        >
          <svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round">
            <path d="M1.5 1.5L9.5 9.5" />
            <path d="M9.5 1.5L1.5 9.5" />
          </svg>
        </button>
      </div>
    </div>
  );
}
