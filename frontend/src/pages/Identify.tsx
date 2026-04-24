import { useState, useCallback } from "react";
import { useDropzone } from "react-dropzone";
import { AudioRecorder } from "@/components/identify/AudioRecorder";
import { identifyApi } from "@/lib/api";
import type { MatchResult } from "@/types";
import { MusicNote, Microphone, Upload, SineWave } from "iconoir-react";
import { Badge } from "@/components/ui/badge";

const getFilenameFromMimeType = (mimeType: string | undefined): string => {
    if (!mimeType) return "recording.unknown";

    const mimeToExtension: Record<string, string> = {
        "audio/wav": ".wav",
        "audio/x-wav": ".wav",
        "audio/webm": ".webm",
        "audio/ogg": ".ogg",
        "audio/mp4": ".m4a",
        "audio/mpeg": ".mp3",
        "audio/aac": ".aac",
        "audio/flac": ".flac",
    };

    for (const [mime, ext] of Object.entries(mimeToExtension)) {
        if (mimeType.startsWith(mime)) {
            return `recording${ext}`;
        }
    }

    return "recording.unknown";
};

export function Identify() {
    const [results, setResults] = useState<MatchResult[] | null>(null);
    const [loading, setLoading] = useState(false);
    const [activeTab, setActiveTab] = useState<"record" | "upload">("record");
    const [error, setError] = useState<string | null>(null);

    const handleIdentify = useCallback(
        async (file: File | Blob, filename?: string) => {
            setLoading(true);
            setError(null);
            setResults(null);

            try {
                const data = await identifyApi.match(file, {
                    filename:
                        filename ??
                        (file instanceof File
                            ? file.name
                            : getFilenameFromMimeType(file.type)),
                });
                setResults(data.matches);
                if (data.matches.length === 0) {
                    setError("No matches found in library.");
                }
            } catch (err: unknown) {
                console.error(err);
                const e = err as {
                    response?: { data?: { error?: string; message?: string } };
                };
                const serverMessage =
                    e.response?.data?.error || e.response?.data?.message;
                setError(
                    serverMessage || "Identification failed. Please try again.",
                );
            } finally {
                setLoading(false);
            }
        },
        [],
    );

    const onDrop = useCallback(
        (accepted: File[], fileRejections: any[]) => {
            if (fileRejections.length > 0) {
                const error = fileRejections[0].errors[0];
                if (error.code === "file-too-large") {
                    setError("File is too large. Max size is 10MB.");
                } else if (error.code === "file-invalid-type") {
                    setError("Invalid file type. Please upload an audio file.");
                } else {
                    setError(error.message);
                }
                return;
            }

            if (accepted[0]) {
                handleIdentify(accepted[0]);
            }
        },
        [handleIdentify],
    );

    const { getRootProps, getInputProps, isDragActive } = useDropzone({
        onDrop,
        accept: {
            "audio/mpeg": [".mp3"],
            "audio/wav": [".wav"],
            "audio/flac": [".flac"],
            "audio/ogg": [".ogg"],
            "application/ogg": [".ogg"],
            "audio/mp4": [".m4a"],
            "audio/x-m4a": [".m4a"],
            "audio/aac": [".aac"],
        },
        multiple: false,
        maxSize: 10 * 1024 * 1024, // 10MB
    });

    return (
        <div className="max-w-2xl mx-auto px-4 py-16 bg-background transition-colors duration-300">
            <div className="text-center mb-12">
                <h1 className="text-4xl font-bold text-foreground tracking-tight mb-3">
                    Identify
                </h1>
                <p className="text-muted-foreground font-medium">
                    Find matching tracks by recording or uploading a sample
                </p>
            </div>

            {/* Tabs */}
            <div className="flex gap-2 p-1.5 bg-muted/30 border-[1.5px] border-border rounded-2xl mb-8 w-fit mx-auto">
                <button
                    onClick={() => setActiveTab("record")}
                    className={`flex items-center gap-2 px-6 py-2 rounded-xl text-sm font-bold transition-all ${
                        activeTab === "record"
                            ? "bg-card text-foreground sketch-shadow border-border border"
                            : "text-muted-foreground hover:text-foreground"
                    }`}
                >
                    <Microphone className="w-4 h-4" />
                    RECORD
                </button>
                <button
                    onClick={() => setActiveTab("upload")}
                    className={`flex items-center gap-2 px-6 py-2 rounded-xl text-sm font-bold transition-all ${
                        activeTab === "upload"
                            ? "bg-card text-foreground sketch-shadow border-border border"
                            : "text-muted-foreground hover:text-foreground"
                    }`}
                >
                    <Upload className="w-4 h-4" />
                    UPLOAD
                </button>
            </div>

            <div className="bg-card border-[1.5px] border-border rounded-3xl p-8 sketch-shadow mb-12">
                {activeTab === "record" ? (
                    <AudioRecorder onRecorded={handleIdentify} />
                ) : (
                    <div
                        {...getRootProps()}
                        className={`border-2 border-dashed rounded-2xl p-12 text-center cursor-pointer transition-all duration-300 ${
                            isDragActive
                                ? "border-primary bg-primary/5"
                                : "border-border/60 hover:border-primary/40 hover:bg-muted/30"
                        }`}
                    >
                        <input {...getInputProps()} />
                        <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mx-auto mb-4 border border-border/40">
                            <Upload className="w-8 h-8 text-muted-foreground" />
                        </div>
                        <p className="text-lg font-bold text-foreground">
                            Drop identification sample
                        </p>
                        <p className="text-xs font-bold uppercase tracking-widest text-muted-foreground mt-2 opacity-60">
                            Audio files up to 15s recommended
                        </p>
                    </div>
                )}
            </div>

            {error && (
                <div className="mb-8 p-4 bg-amber-500/10 border border-amber-500/20 rounded-xl text-amber-600 dark:text-amber-400 text-sm font-medium text-center">
                    {error}
                </div>
            )}

            {loading && (
                <div className="flex flex-col items-center gap-4 py-12">
                    <SineWave className="w-12 h-12 text-primary animate-pulse" />
                    <p className="text-sm font-bold uppercase tracking-widest text-muted-foreground animate-pulse">
                        Analyzing Ink...
                    </p>
                </div>
            )}

            {results && results.length > 0 && (
                <div className="space-y-4 mt-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
                    <h3 className="text-xs font-bold text-muted-foreground uppercase tracking-[0.2em] mb-6 pl-1">
                        {results.length} match{results.length !== 1 ? "es" : ""}{" "}
                        found
                    </h3>
                    {results.map((match, i) => (
                        <div
                            key={i}
                            className="flex items-center gap-4 p-5 border-[1.5px] border-border rounded-2xl bg-card transition-all hover:border-primary/30 sketch-shadow-hover"
                        >
                            <div className="w-14 h-14 rounded-xl bg-muted/50 border border-border/50 flex items-center justify-center">
                                <MusicNote className="w-7 h-7 text-muted-foreground" />
                            </div>
                            <div className="flex-1">
                                <h4 className="font-bold text-lg text-foreground leading-tight">
                                    {match.track.title}
                                </h4>
                                <p className="text-sm text-muted-foreground font-medium">
                                    {match.track.artist || "Unknown artist"}
                                </p>
                            </div>
                            <div className="text-right">
                                <Badge
                                    variant="secondary"
                                    className="mb-1 bg-primary/10 text-primary border-primary/20 hover:bg-primary/20 transition-colors"
                                >
                                    {(match.confidence * 100).toFixed(0)}% match
                                </Badge>
                                <p className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground opacity-60">
                                    at {Math.floor(match.offset_secs / 60)}:
                                    {String(
                                        Math.floor(match.offset_secs % 60),
                                    ).padStart(2, "0")}
                                </p>
                            </div>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
