import { Link } from "react-router-dom";
import { MusicNote, Microphone, PlusCircle, Bell, SunLight, HalfMoon } from "iconoir-react";
import { Badge } from "@/components/ui/badge";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

interface NavbarProps {
    unreadCount: number;
    notifications: { track_id: string; message?: string; status: string }[];
    onClear: () => void;
    theme: "light" | "dark";
    onToggleTheme: () => void;
}

export function Navbar({ unreadCount, notifications, onClear, theme, onToggleTheme }: NavbarProps) {
    return (
        <nav className="border-b-[1.5px] border-border bg-background/60 backdrop-blur-xl sticky top-0 z-50">
            <div className="max-w-7xl mx-auto px-4 h-16 flex items-center justify-between">
                <Link
                    to="/"
                    className="flex items-center gap-2 text-foreground"
                >
                    <MusicNote className="w-6 h-6" strokeWidth={1.5} />
                    <span className="font-semibold text-lg tracking-tight">
                        Auris
                    </span>
                </Link>

                <div className="flex items-center gap-6">
                    <Link
                        to="/"
                        className="text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
                    >
                        Library
                    </Link>
                    <Link
                        to="/identify"
                        className="flex items-center gap-1.5 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
                    >
                        <Microphone className="w-4 h-4" />
                        Identify
                    </Link>
                    <Link
                        to="/upload"
                        className="flex items-center gap-1.5 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
                    >
                        <PlusCircle className="w-4 h-4" />
                        Upload
                    </Link>

                    <button
                        onClick={onToggleTheme}
                        className="p-2 hover:bg-accent rounded-full transition-colors text-muted-foreground hover:text-foreground"
                        aria-label="Toggle theme"
                    >
                        {theme === "light" ? (
                            <HalfMoon className="w-5 h-5" />
                        ) : (
                            <SunLight className="w-5 h-5" />
                        )}
                    </button>

                    <DropdownMenu>
                        <DropdownMenuTrigger className="relative p-2 hover:bg-accent rounded-full transition-colors">
                            <Bell
                                className="w-5 h-5 text-muted-foreground"
                                strokeWidth={1.5}
                            />
                            {unreadCount > 0 && (
                                <Badge
                                    variant="destructive"
                                    className="absolute -top-1 -right-1 h-5 w-5 flex items-center justify-center p-0 text-[10px]"
                                >
                                    {unreadCount}
                                </Badge>
                            )}
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-80">
                            {notifications.length === 0 ? (
                                <div className="p-4 text-sm text-neutral-500 text-center">
                                    No notifications
                                </div>
                            ) : (
                                <>
                                    <div className="flex items-center justify-between px-3 py-2 border-b border-neutral-100">
                                        <span className="text-xs font-medium text-neutral-500">
                                            Notifications
                                        </span>
                                        <button
                                            onClick={onClear}
                                            className="text-xs text-neutral-400 hover:text-neutral-600"
                                        >
                                            Clear
                                        </button>
                                    </div>
                                    {notifications.slice(0, 5).map((n, i) => (
                                        <DropdownMenuItem
                                            key={i}
                                            className="flex flex-col items-start gap-1 py-3"
                                        >
                                            <span className="text-sm font-medium text-neutral-800">
                                                {n.status === "completed"
                                                    ? "✅ Ready"
                                                    : n.status === "failed"
                                                      ? "❌ Failed"
                                                      : "⏳ Processing"}
                                            </span>
                                            <span className="text-xs text-neutral-500 truncate w-full">
                                                {n.message ||
                                                    `Track ${n.track_id.slice(0, 8)}...`}
                                            </span>
                                        </DropdownMenuItem>
                                    ))}
                                </>
                            )}
                        </DropdownMenuContent>
                    </DropdownMenu>
                </div>
            </div>
        </nav>
    );
}
