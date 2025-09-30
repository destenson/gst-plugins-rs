import {
  ChartBarIcon,
  Cog6ToothIcon,
  DocumentTextIcon,
  FolderIcon,
  HomeIcon,
  QuestionMarkCircleIcon,
  VideoCameraIcon,
} from "@heroicons/react/24/outline";
import { ComponentType } from "react";

export interface NavItem {
  name: string;
  path: string;
  icon: ComponentType<{ className?: string }>;
  description?: string;
}

export const navigationItems: NavItem[] = [
  {
    name: "Dashboard",
    path: "/",
    icon: HomeIcon,
    description: "System overview and status",
  },
  {
    name: "Streams",
    path: "/streams",
    icon: VideoCameraIcon,
    description: "Manage video streams",
  },
  {
    name: "Recordings",
    path: "/recordings",
    icon: FolderIcon,
    description: "Browse and manage recordings",
  },
  {
    name: "Configuration",
    path: "/configuration",
    icon: Cog6ToothIcon,
    description: "System settings",
  },
  {
    name: "Metrics",
    path: "/metrics",
    icon: ChartBarIcon,
    description: "Performance monitoring",
  },
  {
    name: "Logs",
    path: "/logs",
    icon: DocumentTextIcon,
    description: "System logs",
  },
  {
    name: "Help",
    path: "/help",
    icon: QuestionMarkCircleIcon,
    description: "Documentation and support",
  },
];
