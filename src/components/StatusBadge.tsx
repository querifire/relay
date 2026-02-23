import type { ProxyStatusInfo } from "../types";

interface Props {
  status: ProxyStatusInfo;
}

export default function StatusBadge({ status }: Props) {
  let label: string;
  let className: string;

  if (status === "Running") {
    label = "Active";
    className = "bg-[rgba(52,199,89,0.1)] text-[#34C759]";
  } else if (status === "Starting") {
    label = "Starting";
    className = "bg-[rgba(255,159,10,0.1)] text-[#FF9F0A]";
  } else if (status === "Stopped") {
    label = "Idle";
    className = "bg-[rgba(142,142,147,0.1)] text-foreground-muted";
  } else {
    label = "Error";
    className = "bg-[rgba(255,59,48,0.1)] text-[#FF3B30]";
  }

  return (
    <span
      className={`inline-flex items-center px-[0.625rem] py-1 rounded-badge text-[0.6875rem] font-semibold tracking-[0.02em] ${className}`}
    >
      {label}
    </span>
  );
}
