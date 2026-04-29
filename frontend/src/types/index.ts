export interface Track {
    id: string;
    title: string;
    artist: string | null;
    duration_secs: number;
    status: "pending" | "fingerprinting" | "ready" | "error";
    created_at: string;
}

export interface MatchResult {
    track: Track;
    confidence: number;
    match_count: number;
    offset_secs: number;
}

export interface JobMessage {
    track_id: string;
    status: string;
    progress?: number;
    message?: string;
    timestamp: string;
}

export interface CreateTrackRequest {
    title: string;
    artist?: string;
}

export interface UpdateTrackRequest {
    title?: string;
    artist?: string;
}

export interface TrackListResponse {
    tracks: Track[];
    total_count: number;
}
