import * as TabsPrimitive from "@radix-ui/react-tabs";
import { cn } from "../../lib/utils";

export const Tabs = TabsPrimitive.Root;

export function TabsList({ className, ...props }: TabsPrimitive.TabsListProps) {
  return <TabsPrimitive.List className={cn("inline-flex h-9 items-center rounded-[8px] border border-border bg-muted p-1", className)} {...props} />;
}

export function TabsTrigger({ className, ...props }: TabsPrimitive.TabsTriggerProps) {
  return (
    <TabsPrimitive.Trigger
      className={cn("inline-flex h-7 items-center justify-center rounded-[6px] px-3 text-xs text-muted-foreground data-[state=active]:bg-primary/15 data-[state=active]:text-primary", className)}
      {...props}
    />
  );
}
