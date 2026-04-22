import { useState } from "react";
import { MusicNote, MoreVert, Trash, EditPencil } from "iconoir-react";
import type { Track } from "@/types";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

interface TrackCardProps {
    track: Track;
    onDelete: (id: string) => void;
    onEdit: (id: string, data: { title: string; artist?: string }) => void;
}

export function TrackCard({ track, onDelete, onEdit }: TrackCardProps) {
    const [editOpen, setEditOpen] = useState(false);
    const [title, setTitle] = useState(track.title);
    const [artist, setArtist] = useState(track.artist || "");

    return (
        <>
            <div className="bg-card border border-border p-0 h-80 w-full flex flex-col">
                <div className="relative h-60 bg-muted/30 overflow-hidden">
                    {/* Cover image area - ready for embedded audio covers when available */}
                    <div className="w-full h-full flex items-center justify-center">
                        <MusicNote
                            className="w-12 h-12 text-muted-foreground/50"
                            strokeWidth={1}
                        />
                    </div>

                    {/* Overlay with menu button */}
                    <div className="absolute top-2 right-2">
                        <DropdownMenu>
                            <DropdownMenuTrigger>
                                <button className="p-1.5 hover:bg-black/20 backdrop-blur-sm rounded-lg transition-opacity opacity-0 group-hover:opacity-100">
                                    <MoreVert className="w-4 h-4 text-white" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                                <DropdownMenuItem
                                    onClick={() => setEditOpen(true)}
                                    className="gap-2"
                                >
                                    <EditPencil className="w-4 h-4" />
                                    Edit metadata
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                    onClick={() => onDelete(track.id)}
                                    className="gap-2 text-destructive focus:text-destructive"
                                >
                                    <Trash className="w-4 h-4" />
                                    Delete
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>
                </div>

                {/* Track Info */}
                <div className="p-4">
                    <h3
                        className="font-medium text-foreground truncate mb-1"
                        title={track.title}
                    >
                        {track.title}
                    </h3>
                    <p className="text-sm text-muted-foreground truncate">
                        {track.artist || "Unknown artist"}
                    </p>
                </div>
            </div>

            <Dialog open={editOpen} onOpenChange={setEditOpen}>
                <DialogContent className="bg-card border-border">
                    <DialogHeader>
                        <DialogTitle className="text-foreground">
                            Edit Track
                        </DialogTitle>
                    </DialogHeader>
                    <div className="space-y-4 pt-4">
                        <div>
                            <label className="text-sm font-medium text-muted-foreground mb-1.5 block">
                                Title
                            </label>
                            <input
                                value={title}
                                onChange={(e) => setTitle(e.target.value)}
                                className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                            />
                        </div>
                        <div>
                            <label className="text-sm font-medium text-muted-foreground mb-1.5 block">
                                Artist
                            </label>
                            <input
                                value={artist}
                                onChange={(e) => setArtist(e.target.value)}
                                className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                            />
                        </div>
                        <div className="flex justify-end gap-2">
                            <Button
                                variant="outline"
                                onClick={() => setEditOpen(false)}
                            >
                                Cancel
                            </Button>
                            <Button
                                onClick={() => {
                                    onEdit(track.id, {
                                        title,
                                        artist: artist || undefined,
                                    });
                                    setEditOpen(false);
                                }}
                            >
                                Save
                            </Button>
                        </div>
                    </div>
                </DialogContent>
            </Dialog>
        </>
    );
}
