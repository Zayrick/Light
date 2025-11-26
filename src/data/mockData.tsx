import { Music, Cast, Bell } from "lucide-react";

export const BLOG_POSTS = [
  {
    id: 1,
    title: "Getting Started with Light",
    summary:
      "Learn how to set up your first device and configure basic effects.",
    date: "Oct 24, 2023",
  },
  {
    id: 2,
    title: "Advanced Effect Creator",
    summary: "Deep dive into creating custom matrix animations.",
    date: "Nov 02, 2023",
  },
  {
    id: 3,
    title: "Community Showcase",
    summary: "Check out the most popular setups from our community this week.",
    date: "Nov 15, 2023",
  },
];

export const PLUGINS = [
  {
    id: 1,
    name: "Audio Visualizer",
    description: "Sync lights with music beat",
    icon: <Music size={18} />,
  },
  {
    id: 2,
    name: "Screen Mirror",
    description: "Extend screen colors to lights",
    icon: <Cast size={18} />,
  },
  {
    id: 3,
    name: "Notifications",
    description: "Flash on new messages",
    icon: <Bell size={18} />,
  },
];

