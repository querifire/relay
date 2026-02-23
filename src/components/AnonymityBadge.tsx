import type { AnonymityLevel } from "../types";

const LEVEL_CONFIG: Record<AnonymityLevel, { label: string; bg: string; text: string }> = {
  Elite: {
    label: "Elite",
    bg: "bg-[rgba(52,199,89,0.12)]",
    text: "text-[#34C759]",
  },
  Anonymous: {
    label: "Anonymous",
    bg: "bg-[rgba(255,159,10,0.12)]",
    text: "text-[#FF9F0A]",
  },
  Transparent: {
    label: "Transparent",
    bg: "bg-[rgba(255,59,48,0.12)]",
    text: "text-[#FF3B30]",
  },
};

interface Props {
  level: AnonymityLevel | null | undefined;
  size?: "sm" | "md";
}

export default function AnonymityBadge({ level, size = "sm" }: Props) {
  if (!level) return null;

  const config = LEVEL_CONFIG[level];
  const sizeClass =
    size === "sm"
      ? "px-1.5 py-0.5 text-[0.5625rem]"
      : "px-2 py-0.5 text-[0.6875rem]";

  return (
    <span
      className={`inline-flex items-center rounded-badge font-semibold ${sizeClass} ${config.bg} ${config.text}`}
    >
      {config.label}
    </span>
  );
}
