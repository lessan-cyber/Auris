import axios from "axios";
import type {
    Track,
    MatchResult,
    UpdateTrackRequest,
    TrackListResponse,
} from "@/types";

const API_URL = import.meta.env.VITE_API_URL;

function getMimeTypeFromFilename(filename: string): string {
    const lower = filename.toLowerCase();

    if (lower.endsWith(".wav")) return "audio/wav";
    if (lower.endsWith(".webm")) return "audio/webm";
    if (lower.endsWith(".ogg")) return "audio/ogg";
    if (lower.endsWith(".m4a") || lower.endsWith(".mp4")) return "audio/mp4";
    if (lower.endsWith(".mp3")) return "audio/mpeg";
    if (lower.endsWith(".aac")) return "audio/aac";
    if (lower.endsWith(".flac")) return "audio/flac";

    return "application/octet-stream";
}

export const api = axios.create({
    baseURL: API_URL,
    timeout: 10000,
});

export const trackApi = {
    checkHealth: () =>
        api
            .get<{
                status: string;
                database: string;
                s3: string;
            }>("/health", { timeout: 5000 })
            .then((r) => r.data),
    list: (limit?: number, page?: number) => {
        const params = new URLSearchParams();
        if (limit !== undefined) params.append("limit", limit.toString());
        if (page !== undefined) params.append("page", page.toString());
        return api
            .get<TrackListResponse>("/tracks", { params })
            .then((r) => r.data);
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
    match: async (
        file: Blob | File,
        options?: {
            filename?: string;
            transparency?: boolean;
            signal?: AbortSignal;
        },
    ) => {
        const filename =
            options?.filename ??
            (file instanceof File ? file.name : "recording.wav");
        const contentType = file.type || getMimeTypeFromFilename(filename);

        return api
            .post<{
                matches: MatchResult[];
                query_duration_ms: number;
                sample_duration_secs: number;
            }>("/identify/raw", file, {
                params: {
                    filename,
                },
                headers: {
                    "Content-Type": contentType,
                },
                timeout: 60000,
                signal: options?.signal,
            })
            .then((r) => r.data);
    },
};
