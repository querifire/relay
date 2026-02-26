interface Props {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  disabled?: boolean;
}

export default function CustomCheckbox({ checked, onChange, label, disabled }: Props) {
  return (
    <label
      className={`flex items-center gap-2 cursor-pointer select-none group ${disabled ? "opacity-50 cursor-not-allowed" : ""}`}
    >
      <button
        type="button"
        role="checkbox"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => !disabled && onChange(!checked)}
        className={`w-4 h-4 rounded-[4px] border flex-shrink-0 flex items-center justify-center transition-colors duration-150 outline-none focus-visible:ring-1 focus-visible:ring-border-focus ${
          checked
            ? "bg-foreground border-foreground"
            : "bg-surface border-border group-hover:border-border-focus"
        }`}
      >
        {checked && (
          <svg
            width="9"
            height="7"
            viewBox="0 0 9 7"
            fill="none"
            className="text-surface dark:text-[#1C1C1E]"
          >
            <path
              d="M1 3.5L3.5 6L8 1"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        )}
      </button>
      {label && (
        <span className="text-[0.8125rem] text-foreground">{label}</span>
      )}
    </label>
  );
}
