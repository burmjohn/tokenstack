import {
  BarChart3,
  Database,
  LayoutDashboard,
  RefreshCcw,
  ServerCog,
  type LucideIcon,
} from "lucide-react";

export type NavSection = "dashboard" | "usage" | "resets" | "sources" | "setup";

export type NavItem = {
  id: NavSection;
  label: string;
  icon: LucideIcon;
};

export const NAV_ITEMS: readonly NavItem[] = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "usage", label: "Usage", icon: BarChart3 },
  { id: "resets", label: "Reset credits", icon: RefreshCcw },
  { id: "sources", label: "Sources", icon: Database },
  { id: "setup", label: "Setup", icon: ServerCog },
] as const;

export const SECTION_COPY: Record<NavSection, { heading: string; description: string }> = {
  dashboard: {
    heading: "Dashboard",
    description: "Local Codex usage, reset credits, and source coverage.",
  },
  usage: {
    heading: "Usage",
    description: "Review imported local usage and session history.",
  },
  resets: {
    heading: "Reset credits",
    description: "Track reset-credit snapshots when they are available.",
  },
  sources: {
    heading: "Sources",
    description: "See which local and remote sources currently have evidence.",
  },
  setup: {
    heading: "Setup",
    description: "Connect local history and refresh available snapshots.",
  },
};
