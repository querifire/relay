import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";

const STORAGE_KEY = "relay_welcomed";

export function hasBeenWelcomed(): boolean {
  return typeof localStorage !== "undefined" && localStorage.getItem(STORAGE_KEY) !== null;
}

export function setWelcomed(): void {
  localStorage.setItem(STORAGE_KEY, "1");
  document.documentElement.removeAttribute("data-splash");
}

interface WelcomeScreenProps {
  onContinue: () => void;
}

const PARTICLES = [
  { x: 14, y: 18, r: 1.5, delay: 0.3, dur: 5 },
  { x: 82, y: 13, r: 2,   delay: 0.9, dur: 6 },
  { x: 76, y: 72, r: 1.5, delay: 1.3, dur: 5.5 },
  { x: 20, y: 77, r: 2,   delay: 0.6, dur: 6.5 },
  { x: 52, y: 8,  r: 1,   delay: 1.6, dur: 5 },
  { x: 91, y: 48, r: 1.5, delay: 0.4, dur: 6 },
  { x: 8,  y: 54, r: 1,   delay: 1.1, dur: 5.5 },
  { x: 62, y: 87, r: 2,   delay: 0.8, dur: 6 },
  { x: 38, y: 90, r: 1.5, delay: 1.9, dur: 5 },
  { x: 89, y: 83, r: 1,   delay: 0.5, dur: 6.5 },
  { x: 44, y: 24, r: 1,   delay: 1.4, dur: 5 },
  { x: 6,  y: 33, r: 1.5, delay: 0.7, dur: 5.5 },
];

const NODES = [
  { cx: 160, cy: 304 },
  { cx: 256, cy: 176 },
  { cx: 352, cy: 304 },
];

const LETTERS = ["R", "e", "l", "a", "y"];

export default function WelcomeScreen({ onContinue }: WelcomeScreenProps) {
  const [visible, setVisible] = useState(true);

  const handleContinue = () => {
    setWelcomed();
    setVisible(false);
  };

  return (
    <AnimatePresence onExitComplete={onContinue}>
      {visible && (
        <motion.div
          key="welcome"
          className="fixed inset-0 z-[100] overflow-hidden flex items-center justify-center select-none"
          initial={{ clipPath: "circle(150% at 50% 50%)" }}
          exit={{ clipPath: "circle(0% at 50% 50%)", transition: { duration: 0.55, ease: [0.4, 0, 0.6, 1] } }}
          style={{
            background: "radial-gradient(ellipse 90% 80% at 50% 50%, #180c08 0%, #0d0608 45%, #060408 100%)",
          }}
        >
          {}
          <div
            className="absolute inset-0 pointer-events-none"
            style={{
              backgroundImage: "radial-gradient(circle, rgba(232,158,107,0.12) 1px, transparent 1px)",
              backgroundSize: "28px 28px",
              maskImage: "radial-gradient(ellipse 70% 70% at 50% 50%, black 30%, transparent 100%)",
            }}
          />

          {}
          <div
            className="absolute inset-0 opacity-[0.035] pointer-events-none"
            style={{
              backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E")`,
              backgroundSize: "180px 180px",
            }}
          />

          {}
          <motion.div
            className="absolute pointer-events-none rounded-full"
            style={{
              width: 600,
              height: 600,
              background: "radial-gradient(circle, rgba(217,91,91,0.12) 0%, rgba(232,158,107,0.06) 45%, transparent 70%)",
              filter: "blur(50px)",
              top: "50%",
              left: "50%",
              translate: "-50% -50%",
            }}
            animate={{ scale: [1, 1.18, 1], opacity: [0.7, 1, 0.7] }}
            transition={{ duration: 5, repeat: Infinity, ease: "easeInOut" }}
          />
          <motion.div
            className="absolute -top-32 -left-32 pointer-events-none rounded-full"
            style={{
              width: 400,
              height: 400,
              background: "radial-gradient(circle, rgba(232,158,107,0.07) 0%, transparent 70%)",
              filter: "blur(60px)",
            }}
            animate={{ x: [0, 20, 0], y: [0, 15, 0] }}
            transition={{ duration: 8, repeat: Infinity, ease: "easeInOut" }}
          />
          <motion.div
            className="absolute -bottom-32 -right-32 pointer-events-none rounded-full"
            style={{
              width: 350,
              height: 350,
              background: "radial-gradient(circle, rgba(217,91,91,0.08) 0%, transparent 70%)",
              filter: "blur(60px)",
            }}
            animate={{ x: [0, -15, 0], y: [0, -20, 0] }}
            transition={{ duration: 9, repeat: Infinity, ease: "easeInOut", delay: 1 }}
          />

          {}
          {PARTICLES.map((p, i) => (
            <motion.div
              key={i}
              className="absolute rounded-full pointer-events-none"
              style={{
                left: `${p.x}%`,
                top: `${p.y}%`,
                width: p.r * 2,
                height: p.r * 2,
                background: "rgba(232,158,107,0.7)",
                boxShadow: `0 0 ${p.r * 5}px rgba(232,158,107,0.5)`,
              }}
              initial={{ opacity: 0, scale: 0 }}
              animate={{
                opacity: [0, 0.8, 0.4, 0.8, 0],
                scale: [0, 1, 0.7, 1, 0],
                y: [0, -10, 0, 10, 0],
              }}
              transition={{
                delay: p.delay,
                duration: p.dur,
                repeat: Infinity,
                ease: "easeInOut",
              }}
            />
          ))}

          {}
          <div className="relative z-10 flex flex-col items-center">

            {}
            <div className="mb-9 relative">
              {}
              <motion.div
                className="absolute pointer-events-none rounded-[1.5rem]"
                style={{
                  inset: -8,
                  background: "linear-gradient(135deg, rgba(232,158,107,0.5), rgba(217,91,91,0.5))",
                  filter: "blur(18px)",
                }}
                animate={{ opacity: [0.4, 0.85, 0.4] }}
                transition={{ duration: 3.5, repeat: Infinity, ease: "easeInOut" }}
              />

              {}
              <motion.div
                className="relative w-[5.5rem] h-[5.5rem] rounded-[1.5rem] flex items-center justify-center"
                style={{
                  background: "linear-gradient(145deg, rgba(232,158,107,0.18) 0%, rgba(217,91,91,0.12) 100%)",
                  border: "1px solid rgba(232,158,107,0.35)",
                  boxShadow: "inset 0 1px 0 rgba(255,255,255,0.08), 0 20px 40px rgba(0,0,0,0.4)",
                }}
                initial={{ opacity: 0, scale: 0.6 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ delay: 0.1, type: "spring", stiffness: 220, damping: 22 }}
              >
                <svg width="52" height="52" viewBox="0 0 512 512" fill="none">
                  <defs>
                    <linearGradient id="wg" x1="0" y1="0" x2="512" y2="512" gradientUnits="userSpaceOnUse">
                      <stop offset="0%" stopColor="#E89E6B" />
                      <stop offset="100%" stopColor="#D95B5B" />
                    </linearGradient>
                  </defs>

                  {}
                  <motion.path
                    d="M 256 64 L 416 128 L 416 256 C 416 368 352 448 256 480 C 160 448 96 368 96 256 L 96 128 Z"
                    stroke="url(#wg)"
                    strokeWidth="26"
                    strokeLinejoin="round"
                    fill="rgba(232,158,107,0.06)"
                    initial={{ pathLength: 0, opacity: 0 }}
                    animate={{ pathLength: 1, opacity: 1 }}
                    transition={{ delay: 0.35, duration: 0.8, ease: [0.4, 0, 0.2, 1] }}
                  />

                  {}
                  <motion.path
                    d="M 160 304 L 256 176 L 352 304"
                    fill="none"
                    stroke="rgba(255,255,255,0.88)"
                    strokeWidth="26"
                    strokeLinejoin="round"
                    strokeLinecap="round"
                    initial={{ pathLength: 0 }}
                    animate={{ pathLength: 1 }}
                    transition={{ delay: 0.85, duration: 0.45, ease: "easeOut" }}
                  />

                  {}
                  {NODES.map((node, i) => (
                    <motion.circle
                      key={i}
                      cx={node.cx}
                      cy={node.cy}
                      r="0"
                      fill="rgba(232,158,107,0.2)"
                      stroke="rgba(255,255,255,0.88)"
                      strokeWidth="14"
                      animate={{ r: 20 }}
                      transition={{
                        delay: 1.05 + i * 0.1,
                        type: "spring",
                        stiffness: 350,
                        damping: 18,
                      }}
                    />
                  ))}
                </svg>
              </motion.div>
            </div>

            {}
            <div className="flex items-baseline mb-3" style={{ gap: "0.02em" }}>
              {LETTERS.map((l, i) => (
                <motion.span
                  key={i}
                  className="text-[3rem] font-bold text-white"
                  style={{ letterSpacing: "0.06em" }}
                  initial={{ opacity: 0, y: 18, filter: "blur(10px)" }}
                  animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
                  transition={{
                    delay: 1.2 + i * 0.06,
                    duration: 0.45,
                    ease: [0.25, 0.1, 0.25, 1],
                  }}
                >
                  {l}
                </motion.span>
              ))}
            </div>

            {}
            <motion.p
              className="text-white/45 text-[0.75rem] font-medium uppercase"
              style={{ letterSpacing: "0.18em" }}
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 1.65, duration: 0.5 }}
            >
              Secure proxy management
            </motion.p>
          </div>

          {}
          <motion.div
            className="absolute bottom-10 right-10"
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 2.0, duration: 0.45 }}
          >
            <motion.button
              onClick={handleContinue}
              whileHover={{ scale: 1.05, borderColor: "rgba(232,158,107,0.6)" }}
              whileTap={{ scale: 0.97 }}
              className="relative flex items-center gap-2.5 px-7 py-[0.7rem] rounded-button overflow-hidden font-semibold text-[0.9rem] text-white/90"
              style={{
                background: "rgba(255,255,255,0.07)",
                border: "1px solid rgba(255,255,255,0.18)",
                backdropFilter: "blur(12px)",
              }}
            >
              {}
              <motion.span
                className="absolute inset-0 pointer-events-none"
                style={{
                  background: "linear-gradient(105deg, transparent 30%, rgba(255,255,255,0.12) 50%, transparent 70%)",
                  translateX: "-100%",
                }}
                animate={{ translateX: ["−100%", "250%"] }}
                transition={{
                  delay: 2.6,
                  duration: 1.0,
                  ease: "easeInOut",
                  repeat: Infinity,
                  repeatDelay: 3.5,
                }}
              />
              Get started
              <motion.svg
                width="14" height="14" viewBox="0 0 24 24"
                fill="none" stroke="currentColor" strokeWidth="2.5"
                strokeLinecap="round" strokeLinejoin="round"
                animate={{ x: [0, 3, 0] }}
                transition={{ delay: 2.2, duration: 1.2, repeat: Infinity, repeatDelay: 2.5, ease: "easeInOut" }}
              >
                <path d="M5 12h14M12 5l7 7-7 7" />
              </motion.svg>
            </motion.button>
          </motion.div>

          {}
          <motion.div
            className="absolute bottom-10 left-10 text-white/20 text-[0.6875rem] font-medium"
            style={{ letterSpacing: "0.06em" }}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 2.2, duration: 0.5 }}
          >
            v0.1.0
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
