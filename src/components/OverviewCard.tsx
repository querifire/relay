import { memo } from "react";
import { motion } from "framer-motion";
import type { ProxyInstanceInfo } from "../types";

interface Props {
  instances: ProxyInstanceInfo[];
  /** Array of 12 values — log counts for the last 12 intervals. */
  trafficData: number[];
}

function OverviewCard({ instances, trafficData }: Props) {
  const runningCount = instances.filter((i) => i.status === "Running").length;
  const totalCount = instances.length;

  // Normalise traffic bars to percentage heights (15–95 range)
  const maxVal = Math.max(...trafficData, 1);
  const barHeights = trafficData.map(
    (v) => 15 + (v / maxVal) * 80,
  );

  // The bar with the highest value gets the accent highlight
  const peakIndex = trafficData.indexOf(Math.max(...trafficData));

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.3, ease: "easeOut" }}
      className="col-span-12 relative overflow-hidden rounded-card border border-border bg-surface-card p-6 flex flex-col justify-between min-h-[12.5rem] shadow-card overview-glow"
    >
      {/* Header */}
      <div className="relative z-10 flex justify-between items-start">
        <div>
          <div className="text-[0.8125rem] text-foreground-muted font-medium mb-1">
            Proxy Instances
          </div>
          <div className="text-[2rem] font-semibold tracking-[-0.02em]">
            {totalCount}{" "}
            <span className="text-[0.875rem] font-normal text-foreground-muted">
              / {runningCount} active
            </span>
          </div>
        </div>
        <div className="text-[0.75rem] text-foreground-muted mt-1">
          Connections (last 12 intervals)
        </div>
      </div>

      {/* Bar chart — connection traffic */}
      <div className="relative z-10 mt-auto h-[3.75rem] flex items-end gap-1">
        {barHeights.map((h, i) => (
          <div
            key={i}
            className={`chart-bar ${i === peakIndex && trafficData[peakIndex] > 0 ? "active" : ""}`}
            style={{ height: `${h}%` }}
          />
        ))}
      </div>
    </motion.div>
  );
}

export default memo(OverviewCard);
