import {
    useState,
    useEffect,
    useContext,
    useRef,
    type Dispatch,
    type SetStateAction,
} from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { trackApi } from "@/lib/api";
import { TrackCard } from "@/components/tracks/TrackCard";
import { SoundMin, NavArrowLeft, NavArrowRight } from "iconoir-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { Track, TrackListResponse } from "@/types";
import { UploadNotificationContext } from "@/lib/contexts";
import type { UploadNotificationContextType } from "@/lib/contexts";
import type { UpdateTrackRequest } from "@/types";
import { toast } from "sonner";

const noopSetUploadNotifications: Dispatch<
    SetStateAction<{ track_id: string; status: string; message?: string }[]>
> = () => {};

export function Library() {
    const [page, setPage] = useState(1);
    const itemsPerPage = 8;
    const queryClient = useQueryClient();

    const uploadNotificationContext = useContext<
        UploadNotificationContextType | undefined
    >(UploadNotificationContext);
    const setUploadNotifications =
        uploadNotificationContext?.setUploadNotifications ??
        noopSetUploadNotifications;

    const { data: trackData, isLoading } = useQuery({
        queryKey: ["tracks", page],
        queryFn: () => trackApi.list(itemsPerPage, page),
    });

    const tracks = (trackData as TrackListResponse)?.tracks || [];
    const totalCount = (trackData as TrackListResponse)?.total_count || 0;

    const deleteMutation = useMutation({
        mutationFn: trackApi.delete,
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ["tracks"] });
        },
    });

    const updateMutation = useMutation({
        mutationFn: ({ id, data }: { id: string; data: UpdateTrackRequest }) =>
            trackApi.update(id, data),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ["tracks"] });
        },
    });

    const totalPages = totalCount ? Math.ceil(totalCount / itemsPerPage) : 1;
    const hasTracks = tracks && tracks.length > 0;
    const pollingToastId = useRef<string | number | null>(null);

    useEffect(() => {
        const trackId = localStorage.getItem("recentUploadTrackId");
        if (!trackId) return;

        let attempts = 0;
        const maxAttempts = 30; // 30 * 2s = 60s max polling time

        const pollInterval = setInterval(async () => {
            attempts++;

            if (attempts > maxAttempts) {
                console.error("Polling timeout for track:", trackId);
                if (pollingToastId.current) {
                    toast.dismiss(pollingToastId.current);
                    pollingToastId.current = null;
                }
                setUploadNotifications((prev) => [
                    {
                        track_id: trackId,
                        status: "error",
                        message:
                            "❌ Polling timed out. Check track status manually.",
                    },
                    ...prev.filter((n) => n.track_id !== trackId),
                ]);
                clearInterval(pollInterval);
                localStorage.removeItem("recentUploadTrackId");
                return;
            }

            try {
                const track = await trackApi.get(trackId);

                // Handle toast for intermediate statuses
                if (
                    track.status === "pending" ||
                    track.status === "fingerprinting"
                ) {
                    const message =
                        track.status === "pending"
                            ? "Upload received, waiting for processing"
                            : "🎵 Inking fingerprints...";

                    if (pollingToastId.current) {
                        toast.loading(message, { id: pollingToastId.current });
                    } else {
                        pollingToastId.current = toast.loading(message);
                    }
                    return;
                }

                // Handle final statuses
                if (track.status === "ready" || track.status === "error") {
                    if (pollingToastId.current) {
                        toast.dismiss(pollingToastId.current);
                        pollingToastId.current = null;
                    }

                    const message =
                        track.status === "ready"
                            ? "✅ Track ready!"
                            : "❌ Processing failed";

                    setUploadNotifications((prev) => {
                        const existingIndex = prev.findIndex(
                            (n) => n.track_id === trackId,
                        );
                        const newNotification = {
                            track_id: trackId,
                            status: track.status,
                            message,
                        };

                        if (existingIndex >= 0) {
                            return prev.map((n, i) =>
                                i === existingIndex ? newNotification : n,
                            );
                        } else {
                            return [newNotification, ...prev];
                        }
                    });

                    clearInterval(pollInterval);
                    localStorage.removeItem("recentUploadTrackId");
                    queryClient.invalidateQueries({ queryKey: ["tracks"] });
                }
            } catch (error) {
                console.error("Polling error:", error);

                if (attempts > maxAttempts / 2) {
                    if (pollingToastId.current) {
                        toast.dismiss(pollingToastId.current);
                        pollingToastId.current = null;
                    }
                    clearInterval(pollInterval);
                    localStorage.removeItem("recentUploadTrackId");
                }
            }
        }, 2000); // Poll every 2 seconds

        return () => {
            clearInterval(pollInterval);
            if (pollingToastId.current) {
                toast.dismiss(pollingToastId.current);
            }
        };
    }, [queryClient, setUploadNotifications]);

    if (isLoading) {
        return (
            <div className="max-w-7xl mx-auto px-4 py-12 bg-background transition-colors duration-300">
                <div className="flex items-end justify-between mb-10 border-b-[1.5px] border-border pb-6">
                    <div>
                        <Skeleton className="h-10 w-48 mb-2" />
                        <Skeleton className="h-4 w-64" />
                    </div>
                    <Skeleton className="h-6 w-20 rounded-full" />
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                    {Array.from({ length: 8 }).map((_, i) => (
                        <div
                            key={i}
                            className="bg-card border border-border p-0 h-80 w-full flex flex-col"
                        >
                            <Skeleton className="h-60 w-full" />
                            <div className="p-4 space-y-2">
                                <Skeleton className="h-5 w-3/4" />
                                <Skeleton className="h-4 w-1/2" />
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        );
    }

    return (
        <div className="max-w-7xl mx-auto px-4 py-16 bg-background transition-colors duration-300">
            <div className="flex flex-col md:flex-row md:items-end justify-between mb-12 gap-6">
                <div>
                    <h1 className="text-4xl font-bold text-foreground tracking-tight mb-3">
                        Library
                    </h1>
                    <p className="text-muted-foreground font-medium">
                        Your collection of fingerprinted tracks
                    </p>
                </div>
                <div className="flex items-center gap-4">
                    <span className="text-xs font-bold uppercase tracking-widest text-muted-foreground bg-muted/50 px-4 py-2 rounded-xl border-[1.5px] border-border">
                        {totalCount || 0} Total Tracks
                    </span>
                </div>
            </div>

            {hasTracks ? (
                <>
                    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6">
                        {tracks?.map((track: Track) => (
                            <TrackCard
                                key={track.id}
                                track={track}
                                onDelete={async (id) => {
                                    await deleteMutation.mutateAsync(id);
                                }}
                                onEdit={async (id, data) => {
                                    await updateMutation.mutateAsync({
                                        id,
                                        data,
                                    });
                                }}
                            />
                        ))}
                    </div>

                    {totalPages > 1 && (
                        <div className="flex items-center justify-center gap-4 mt-12">
                            <Button
                                variant="outline"
                                onClick={() =>
                                    setPage((p) => Math.max(1, p - 1))
                                }
                                disabled={page === 1}
                                className="px-4 py-2 rounded-xl font-bold text-xs uppercase tracking-widest sketch-shadow-hover transition-all disabled:opacity-50 disabled:translate-0 disabled:shadow-none"
                            >
                                <NavArrowLeft className="w-4 h-4 mr-2" />
                                Previous
                            </Button>
                            <span className="text-sm font-bold text-foreground bg-muted/30 px-4 py-2 rounded-xl border-[1.5px] border-border">
                                {page} / {totalPages}
                            </span>
                            <Button
                                variant="outline"
                                onClick={() =>
                                    setPage((p) => Math.min(totalPages, p + 1))
                                }
                                disabled={page === totalPages}
                                className="px-4 py-2 rounded-xl font-bold text-xs uppercase tracking-widest sketch-shadow-hover transition-all disabled:opacity-50 disabled:translate-0 disabled:shadow-none"
                            >
                                Next
                                <NavArrowRight className="w-4 h-4 ml-2" />
                            </Button>
                        </div>
                    )}
                </>
            ) : (
                <div className="flex flex-col items-center justify-center py-24 text-muted-foreground">
                    <SoundMin
                        className="w-12 h-12 mb-4 opacity-20"
                        strokeWidth={1}
                    />
                    <p className="text-lg font-medium text-foreground">
                        No tracks yet
                    </p>
                    <p className="text-sm">Upload a song to get started</p>
                </div>
            )}
        </div>
    );
}
