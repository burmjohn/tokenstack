import * as React from "react";
import { cn } from "../../lib/utils";

type BadgeTone = "default" | "success" | "warning" | "source" | "muted";

const tones: Record<BadgeTone, string> = {
  default: "border-border bg-secondary text-secondary-foreground",
  success: "border-mint/40 bg-mint/10 text-mint",
  warning: "border-amber/40 bg-amber/10 text-amber",
  source: "border-primary/40 bg-primary/10 text-primary",
  muted: "border-border bg-muted text-muted-foreground",
};

export function Badge({ className, tone = "default", ...props }: React.HTMLAttributes<HTMLSpanElement> & { tone?: BadgeTone }) {
  return <span className={cn("inline-flex items-center rounded-[6px] border px-2 py-0.5 text-xs font-medium", tones[tone], className)} {...props} />;
}
