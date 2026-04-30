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
    onEdit: (
        id: string,
        data: { title: string; artist?: string },
    ) => Promise<void>;
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
        const toastId = toast.loading("🔗 Generating track link...");
        try {
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
            <div className="bg-card border-[1.5px] border-border rounded-3xl p-0 h-full w-full flex flex-col group sketch-shadow hover:-translate-x-0.5 hover:-translate-y-0.5 transition-all duration-300">
                <div className="relative aspect-4/3 bg-muted/20 rounded-t-[1.35rem] overflow-hidden border-b-[1.5px] border-border/50">
                    <div className="w-full h-full flex items-center justify-center transition-transform duration-500 group-hover:scale-110">
                        <MusicNote
                            className="w-16 h-16 text-muted-foreground/30"
                            strokeWidth={1}
                        />
                    </div>

                    <div className="absolute top-3 right-3">
                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <button className="p-2 bg-background/80 hover:bg-background backdrop-blur-md rounded-xl border border-border/50 transition-all shadow-sm">
                                    <MoreVert className="w-4 h-4 text-foreground" />
                                </button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent
                                align="end"
                                className="rounded-xl border-[1.5px] border-border sketch-shadow"
                            >
                                <DropdownMenuItem
                                    onClick={() => setDetailsOpen(true)}
                                    className="gap-3 py-2.5 cursor-pointer"
                                >
                                    <InfoCircle className="w-4 h-4 text-muted-foreground" />
                                    <span className="font-medium">Details</span>
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                    onClick={() => setEditOpen(true)}
                                    className="gap-3 py-2.5 cursor-pointer"
                                >
                                    <EditPencil className="w-4 h-4 text-muted-foreground" />
                                    <span className="font-medium">
                                        Edit metadata
                                    </span>
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                    onClick={() => setDeleteConfirmOpen(true)}
                                    className="gap-3 py-2.5 text-destructive focus:text-destructive cursor-pointer"
                                >
                                    <Trash className="w-4 h-4" />
                                    <span className="font-medium">Delete</span>
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </div>

                    {track.status !== "ready" && (
                        <div className="absolute bottom-3 left-3">
                            <div className="px-3 py-1 bg-background/90 backdrop-blur-md border border-border/50 rounded-full flex items-center gap-2">
                                <div className="w-2 h-2 rounded-full bg-primary animate-pulse" />
                                <span className="text-[10px] font-bold uppercase tracking-widest text-foreground">
                                    {track.status}
                                </span>
                            </div>
                        </div>
                    )}
                </div>

                <div className="p-5 flex-1 flex flex-col justify-between">
                    <div className="mb-4">
                        <h3
                            className="font-bold text-foreground text-lg leading-tight truncate mb-1"
                            title={track.title}
                        >
                            {track.title}
                        </h3>
                        <p className="text-sm font-medium text-muted-foreground truncate opacity-80">
                            {track.artist || "Unknown artist"}
                        </p>
                    </div>

                    <div className="flex items-center justify-between mt-auto">
                        <span className="text-[10px] font-bold uppercase tracking-widest text-muted-foreground/60">
                            {formatDuration(track.duration_secs)}
                        </span>
                        <button
                            onClick={() => setDetailsOpen(true)}
                            className="text-[10px] font-bold uppercase tracking-widest text-primary hover:underline"
                        >
                            View Details
                        </button>
                    </div>
                </div>
            </div>

            <Dialog open={detailsOpen} onOpenChange={setDetailsOpen}>
                <DialogContent className="bg-card border-[1.5px] border-border rounded-3xl sketch-shadow max-w-md">
                    <DialogHeader>
                        <DialogTitle className="text-foreground text-2xl font-bold">
                            Track Details
                        </DialogTitle>
                    </DialogHeader>
                    <div className="space-y-5 pt-4">
                        <div className="space-y-1.5">
                            <label className="text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground ml-1">
                                ID
                            </label>
                            <div className="w-full px-4 py-3 bg-muted/30 border-[1.5px] border-border rounded-xl text-sm text-foreground break-all font-mono">
                                {track.id}
                            </div>
                        </div>
                        <div className="space-y-1.5">
                            <label className="text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground ml-1">
                                Title
                            </label>
                            <div className="w-full px-4 py-3 bg-muted/30 border-[1.5px] border-border rounded-xl text-sm text-foreground font-bold">
                                {track.title}
                            </div>
                        </div>
                        <div className="space-y-1.5">
                            <label className="text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground ml-1">
                                Artist
                            </label>
                            <div className="w-full px-4 py-3 bg-muted/30 border-[1.5px] border-border rounded-xl text-sm text-foreground font-bold">
                                {track.artist || "Unknown artist"}
                            </div>
                        </div>
                        <div className="flex gap-4">
                            <div className="flex-1 space-y-1.5">
                                <label className="text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground ml-1">
                                    Duration
                                </label>
                                <div className="w-full px-4 py-3 bg-muted/30 border-[1.5px] border-border rounded-xl text-sm text-foreground font-bold text-center">
                                    {formatDuration(track.duration_secs)}
                                </div>
                            </div>
                            <div className="flex-1 space-y-1.5">
                                <label className="text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground ml-1">
                                    Status
                                </label>
                                <div className="w-full px-4 py-3 bg-muted/30 border-[1.5px] border-border rounded-xl text-sm text-foreground font-bold text-center uppercase tracking-widest">
                                    {track.status}
                                </div>
                            </div>
                        </div>
                        <div className="flex justify-end pt-2">
                            <Button
                                variant="outline"
                                onClick={handleCopyTrackLink}
                                className="rounded-xl font-bold text-xs uppercase tracking-widest sketch-shadow-hover transition-all"
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
                <DialogContent className="bg-card border-[1.5px] border-border rounded-3xl sketch-shadow">
                    <DialogHeader>
                        <DialogTitle className="text-foreground text-2xl font-bold">
                            Delete Track
                        </DialogTitle>
                    </DialogHeader>
                    <div className="space-y-6 pt-4">
                        <p className="text-muted-foreground font-medium">
                            Are you sure you want to delete{" "}
                            <span className="text-foreground font-bold">
                                "{track.title}"
                            </span>
                            ? This action cannot be undone.
                        </p>
                        <div className="flex justify-end gap-3">
                            <Button
                                variant="outline"
                                onClick={() => setDeleteConfirmOpen(false)}
                                disabled={isDeleting}
                                className="rounded-xl font-bold text-xs uppercase tracking-widest"
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
                                        toast.success(
                                            "✅ Track deleted successfully",
                                        );
                                    } catch (error) {
                                        console.error(
                                            "Failed to delete track:",
                                            error,
                                        );
                                        toast.error(
                                            "❌ Failed to delete track",
                                            {
                                                description:
                                                    "Please try again later",
                                            },
                                        );
                                    } finally {
                                        setIsDeleting(false);
                                    }
                                }}
                                className="rounded-xl font-bold text-xs uppercase tracking-widest sketch-shadow-hover"
                            >
                                {isDeleting
                                    ? "Deleting..."
                                    : "Delete Permanently"}
                            </Button>
                        </div>
                    </div>
                </DialogContent>
            </Dialog>

            {/* Edit Dialog */}
            <Dialog open={editOpen} onOpenChange={setEditOpen}>
                <DialogContent className="bg-card border-[1.5px] border-border rounded-3xl sketch-shadow">
                    <DialogHeader>
                        <DialogTitle className="text-foreground text-2xl font-bold">
                            Edit Track
                        </DialogTitle>
                    </DialogHeader>
                    <div className="space-y-6 pt-4">
                        <div className="space-y-1.5">
                            <label className="text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground ml-1">
                                Title
                            </label>
                            <input
                                value={title}
                                onChange={(e) => setTitle(e.target.value)}
                                disabled={isSaving}
                                className="w-full px-4 py-3 bg-muted/30 border-[1.5px] border-border rounded-xl text-sm text-foreground font-bold focus:outline-none focus:ring-2 focus:ring-primary/40 transition-all disabled:opacity-50"
                            />
                        </div>
                        <div className="space-y-1.5">
                            <label className="text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground ml-1">
                                Artist
                            </label>
                            <input
                                value={artist}
                                onChange={(e) => setArtist(e.target.value)}
                                disabled={isSaving}
                                className="w-full px-4 py-3 bg-muted/30 border-[1.5px] border-border rounded-xl text-sm text-foreground font-bold focus:outline-none focus:ring-2 focus:ring-primary/40 transition-all disabled:opacity-50"
                            />
                        </div>
                        <div className="flex justify-end gap-3">
                            <Button
                                variant="outline"
                                onClick={() => setEditOpen(false)}
                                disabled={isSaving}
                                className="rounded-xl font-bold text-xs uppercase tracking-widest"
                            >
                                Cancel
                            </Button>
                            <Button
                                disabled={isSaving || !title.trim()}
                                onClick={async () => {
                                    setIsSaving(true);
                                    try {
                                        await onEdit(track.id, {
                                            title: title.trim(),
                                            artist: artist.trim() || undefined,
                                        });
                                        setEditOpen(false);
                                        toast.success(
                                            "✅ Track updated successfully",
                                        );
                                    } catch (error) {
                                        console.error(
                                            "Failed to update track:",
                                            error,
                                        );
                                        toast.error(
                                            "❌ Failed to update track",
                                        );
                                    } finally {
                                        setIsSaving(false);
                                    }
                                }}
                                className="rounded-xl font-bold text-xs uppercase tracking-widest sketch-shadow-hover"
                            >
                                {isSaving ? "Saving..." : "Save Changes"}
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
