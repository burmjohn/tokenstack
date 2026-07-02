import * as ProgressPrimitive from "@radix-ui/react-progress";
import { cn } from "../../lib/utils";

export function Progress({ value, className }: { value: number; className?: string }) {
  return (
    <ProgressPrimitive.Root className={cn("relative h-2 w-full overflow-hidden rounded-full bg-muted", className)} value={value}>
      <ProgressPrimitive.Indicator className="h-full rounded-full bg-primary transition-transform" style={{ transform: `translateX(-${100 - value}%)` }} />
    </ProgressPrimitive.Root>
  );
}
