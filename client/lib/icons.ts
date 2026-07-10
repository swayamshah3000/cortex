/**
 * Resolves Lucide icon name strings (from Space.icon) to actual Lucide React components.
 *
 * Usage:
 *   const Icon = resolveIcon("Home");
 *   <Icon size={20} />
 */

import {
  Home,
  Briefcase,
  Users,
  Receipt,
  Heart,
  Shield,
  FileText,
  Folder,
  Star,
  Tag,
  Calendar,
  DollarSign,
  GraduationCap,
  Car,
  Plane,
  ShoppingCart,
  Music,
  Camera,
  Book,
  Mail,
  Globe,
  Wrench,
  Zap,
  type LucideIcon,
} from "lucide-react";

const iconMap: Record<string, LucideIcon> = {
  Home,
  Briefcase,
  Users,
  Receipt,
  Heart,
  Shield,
  FileText,
  Folder,
  Star,
  Tag,
  Calendar,
  DollarSign,
  GraduationCap,
  Car,
  Plane,
  ShoppingCart,
  Music,
  Camera,
  Book,
  Mail,
  Globe,
  Wrench,
  Zap,
};

/**
 * Maps a Lucide icon name string to the corresponding component.
 * Falls back to FileText if the name is not recognized.
 */
export function resolveIcon(iconName: string): LucideIcon {
  return iconMap[iconName] ?? FileText;
}
