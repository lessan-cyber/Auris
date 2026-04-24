import { useState } from "react";
import {
    MusicNote,
    MoreVert,
    Trash,
    EditPencil,
    InfoCircle,
} from "iconoir-react";
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
import { toast } from "sonner";
import { trackApi } from "@/lib/api";

interface TrackCardProps {
    track: Track;
    onDelete: (id: string) => Promise<void>;
    onEdit: (id: string, data: { title: string; artist?: string }) => Promise<void>;
}

export function TrackCard({ track, onDelete, onEdit }: TrackCardProps) {
    const [editOpen, setEditOpen] = useState(false);
    const [detailsOpen, setDetailsOpen] = useState(false);
    const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
    const [title, setTitle] = useState(track.title);
    const [artist, setArtist] = useState(track.artist || "");
    const [isDeleting, setIsDeleting] = useState(false);
    const [isSaving, setIsSaving] = useState(false);

    const handleCopyTrackLink = async () => {
        try {
            const toastId = toast.loading("🔗 Generating track link...");

            const response = await trackApi.getPresignedUrl(track.id);
            const signedUrl = response.url;

            await navigator.clipboard.writeText(signedUrl);

            toast.success("📋 Track link copied to clipboard!", {
                id: toastId,
                description: "Link copied successfully",
            });
        } catch (error) {
            console.error("Failed to get or copy track link:", error);
            toast.error("❌ Failed to get track link", {
                description: "Please try again later",
            });
        }
    };

    return (
        <>
            <div className="bg-card border border-border p-0 h-80 w-full flex flex-col group">
                <div className="relative h-60 bg-muted/30 overflow-hidden">
                    <div className="w-full h-full flex items-center justify-center">
                        <MusicNote
                            className="w-12 h-12 text-muted-foreground/50"
                            strokeWidth={1}
                        />
                    </div>

                    <div className="absolute top-2 right-2">
                        <DropdownMenu>
                            <DropdownMenuTrigger>
                                <button className="p-1.5 hover:bg-black/20 backdrop-blur-sm rounded-lg transition-opacity">
                                    <MoreVert className="w-4 h-4 text-foreground" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                                <DropdownMenuItem
                                    onClick={() => setDetailsOpen(true)}
                                    className="gap-2"
                                >
                                    <InfoCircle className="w-4 h-4" />
                                    Details
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                    onClick={() => setEditOpen(true)}
                                    className="gap-2"
                                >
                                    <EditPencil className="w-4 h-4" />
                                    Edit metadata
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                    onClick={() => setDeleteConfirmOpen(true)}
                                    className="gap-2 text-destructive focus:text-destructive"
                                >
                                    <Trash className="w-4 h-4" />
                                    Delete
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>
                </div>

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

            <Dialog open={detailsOpen} onOpenChange={setDetailsOpen}>
                <DialogContent className="bg-card border-border max-w-md">
                    <DialogHeader>
                        <DialogTitle className="text-foreground">
                            Track Details
                        </DialogTitle>
                    </DialogHeader>
                    <div className="space-y-4 pt-4">
                        <div>
                            <label className="text-sm font-medium text-muted-foreground mb-1.5 block">
                                ID
                            </label>
                            <div className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm text-foreground break-all">
                                {track.id}
                            </div>
                        </div>
                        <div>
                            <label className="text-sm font-medium text-muted-foreground mb-1.5 block">
                                Title
                            </label>
                            <div className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm text-foreground">
                                {track.title}
                            </div>
                        </div>
                        <div>
                            <label className="text-sm font-medium text-muted-foreground mb-1.5 block">
                                Artist
                            </label>
                            <div className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm text-foreground">
                                {track.artist || "Unknown artist"}
                            </div>
                        </div>
                        <div>
                            <label className="text-sm font-medium text-muted-foreground mb-1.5 block">
                                Duration
                            </label>
                            <div className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm text-foreground">
                                {formatDuration(track.duration_secs)}
                            </div>
                        </div>
                        <div>
                            <label className="text-sm font-medium text-muted-foreground mb-1.5 block">
                                Status
                            </label>
                            <div className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm text-foreground">
                                {track.status}
                            </div>
                        </div>
                        <div className="flex justify-end">
                            <Button
                                variant="outline"
                                onClick={handleCopyTrackLink}
                            >
                                Copy Track Link
                            </Button>
                        </div>
                    </div>
                </DialogContent>
            </Dialog>

            <Dialog
                open={deleteConfirmOpen}
                onOpenChange={setDeleteConfirmOpen}
            >
                <DialogContent className="bg-card border-border">
                    <DialogHeader>
                        <DialogTitle className="text-foreground">
                            Delete Track
                        </DialogTitle>
                    </DialogHeader>
                    <div className="space-y-4 pt-4">
                        <p className="text-sm text-muted-foreground">
                            Are you sure you want to delete "{track.title}"?
                            This action cannot be undone.
                        </p>
                        <div className="flex justify-end gap-2">
                            <Button
                                variant="outline"
                                onClick={() => setDeleteConfirmOpen(false)}
                                disabled={isDeleting}
                            >
                                Cancel
                            </Button>
                            <Button
                                variant="destructive"
                                disabled={isDeleting}
                                onClick={async () => {
                                    setIsDeleting(true);
                                    try {
                                        await onDelete(track.id);
                                        setDeleteConfirmOpen(false);
                                    } catch (error) {
                                        console.error("Failed to delete track:", error);
                                        toast.error("❌ Failed to delete track");
                                    } finally {
                                        setIsDeleting(false);
                                    }
                                }}
                            >
                                {isDeleting ? "Deleting..." : "Delete"}
                            </Button>
                        </div>
                    </div>
                </DialogContent>
            </Dialog>

            {/* Edit Dialog */}
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
                                disabled={isSaving}
                                className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
                            />
                        </div>
                        <div>
                            <label className="text-sm font-medium text-muted-foreground mb-1.5 block">
                                Artist
                            </label>
                            <input
                                value={artist}
                                onChange={(e) => setArtist(e.target.value)}
                                disabled={isSaving}
                                className="w-full px-3 py-2 bg-background border border-border rounded-lg text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
                            />
                        </div>
                        <div className="flex justify-end gap-2">
                            <Button
                                variant="outline"
                                onClick={() => setEditOpen(false)}
                                disabled={isSaving}
                            >
                                Cancel
                            </Button>
                            <Button
                                disabled={isSaving || !title}
                                onClick={async () => {
                                    setIsSaving(true);
                                    try {
                                        await onEdit(track.id, {
                                            title,
                                            artist: artist || undefined,
                                        });
                                        setEditOpen(false);
                                        toast.success("✅ Track updated successfully");
                                    } catch (error) {
                                        console.error("Failed to update track:", error);
                                        toast.error("❌ Failed to update track");
                                    } finally {
                                        setIsSaving(false);
                                    }
                                }}
                            >
                                {isSaving ? "Saving..." : "Save"}
                            </Button>
                        </div>
                    </div>
                </DialogContent>
            </Dialog>
        </>
    );
}

function formatDuration(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, "0")}`;
}
