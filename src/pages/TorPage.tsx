export default function TorPage() {
  return (
    <div>
      {/* Header */}
      <header className="mb-10">
        <div className="flex gap-2 text-foreground-muted text-[0.8125rem] mb-3 items-center">
          <span>Home</span>
          <span>/</span>
          <span className="text-foreground">Security</span>
        </div>
        <h1 className="text-[2rem] font-semibold tracking-[-0.03em]">
          Security
        </h1>
      </header>

      {/* Content */}
      <div className="flex flex-col items-center justify-center py-20 text-center">
        <div className="w-16 h-16 mb-6 rounded-card bg-surface-hover border border-border flex items-center justify-center">
          <svg
            width="32"
            height="32"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            className="text-foreground-muted"
          >
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
          </svg>
        </div>

        <h2 className="text-[1.125rem] font-semibold mb-2">
          Tor Integration
        </h2>

        <p className="text-[0.875rem] text-foreground-muted max-w-md leading-relaxed">
          Tor integration is currently in development. This feature will allow
          you to route traffic through the Tor network for enhanced privacy.
        </p>

        <span className="mt-5 inline-block px-4 py-1.5 text-[0.6875rem] font-semibold text-foreground-tertiary bg-surface-hover border border-border rounded-badge">
          Coming soon
        </span>
      </div>
    </div>
  );
}
