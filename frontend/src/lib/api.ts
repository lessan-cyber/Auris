import axios from "axios";
import type { Track, MatchResult, UpdateTrackRequest } from "@/types";

const API_URL = import.meta.env.VITE_API_URL || "http://localhost:8000";

export const api = axios.create({
    baseURL: API_URL,
});

interface TrackListResponse {
    tracks: Track[];
    total_count: number;
}

export const trackApi = {
    checkHealth: () =>
        api.get<{ status: string; database: string; s3: string }>("/health", { timeout: 5000 }).then((r) => r.data),
    list: (limit?: number, page?: number) => {
        const params = new URLSearchParams();
        if (limit !== undefined) params.append('limit', limit.toString());
        if (page !== undefined) params.append('page', page.toString());
        return api.get<TrackListResponse>("/tracks", { params }).then((r) => r.data);
    },
    get: (id: string) => api.get<Track>(`/tracks/${id}`).then((r) => r.data),
    create: (data: FormData) =>
        api.post<Track>("/tracks", data).then((r) => r.data),
    update: (id: string, data: UpdateTrackRequest) =>
        api.patch<Track>(`/tracks/${id}`, data).then((r) => r.data),
    delete: (id: string) => api.delete(`/tracks/${id}`),
    getPresignedUrl: (id: string) =>
        api.get<{ url: string }>(`/tracks/${id}/url`).then((r) => r.data),
};

export const identifyApi = {
    match: (file: File, transparency = false) => {
        const form = new FormData();
        form.append("file", file);
        return api
            .post<{
                matches: MatchResult[];
                query_duration_ms: number;
            }>(`/identify${transparency ? "?transparency=true" : ""}`, form)
            .then((r) => r.data);
    },
};
