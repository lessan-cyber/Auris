import { useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useDropzone } from "react-dropzone";
import { Upload as UploadIcon, SineWave } from "iconoir-react";
import { Button } from "@/components/ui/button";
import { trackApi } from "@/lib/api";

export function Upload() {
    const navigate = useNavigate();
    const [file, setFile] = useState<File | null>(null);
    const [title, setTitle] = useState("");
    const [artist, setArtist] = useState("");
    const [uploading, setUploading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const onDrop = useCallback((accepted: File[], rejected: any[]) => {
        if (rejected.length > 0) {
            setError("Unsupported file type. Please use MP3, WAV, FLAC, OGG, M4A, or AAC.");
            return;
        }
        if (accepted[0]) {
            setFile(accepted[0]);
            setError(null);
            setTitle(accepted[0].name.replace(/\.[^/.]+$/, ""));
        }
    }, []);

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
    });

    const handleSubmit = async () => {
        if (!file || !title) return;
        setUploading(true);

        const form = new FormData();
        form.append("file", file);
        form.append("title", title);
        if (artist) form.append("artist", artist);

        try {
            const track = await trackApi.create(form);
            
            // Store track ID in localStorage for polling on Library page
            localStorage.setItem("recentUploadTrackId", track.id);
            
            // Navigate immediately to library
            navigate("/");
                
        } catch (e) {
            console.error(e);
            setUploading(false);
        }
    };

    return (
        <div className="max-w-xl mx-auto px-4 py-16 bg-background transition-colors duration-300">
            <div className="text-center mb-12">
                <h1 className="text-4xl font-bold text-foreground tracking-tight mb-3">
                    Upload
                </h1>
                <p className="text-muted-foreground font-medium">
                    Add new tracks to your library for fingerprinting
                </p>
            </div>

            <div className="bg-card border-[1.5px] border-border rounded-3xl p-8 sketch-shadow">
                {error && (
                    <div className="mb-6 p-4 bg-red-500/10 border border-red-500/20 rounded-xl text-red-500 text-sm font-medium animate-in fade-in slide-in-from-top-2">
                        {error}
                    </div>
                )}
                <div
                    {...getRootProps()}
                    className={`border-2 border-dashed rounded-2xl p-12 text-center cursor-pointer transition-all duration-300 ${
                        isDragActive
                            ? "border-primary bg-primary/5"
                            : "border-border/60 hover:border-primary/40 hover:bg-muted/30"
                    }`}
                >
                    <input {...getInputProps()} />
                    <div className="w-16 h-16 bg-muted rounded-full flex items-center justify-center mx-auto mb-6 border border-border/40">
                        <UploadIcon className="w-8 h-8 text-muted-foreground" />
                    </div>
                    {file ? (
                        <div className="bg-primary/10 text-primary px-4 py-2 rounded-full inline-block font-bold text-xs uppercase tracking-widest border border-primary/20">
                            {file.name}
                        </div>
                    ) : (
                        <>
                            <p className="text-lg font-bold text-foreground">
                                Drop audio file here
                            </p>
                            <p className="text-xs font-bold uppercase tracking-widest text-muted-foreground mt-2 opacity-60">
                                MP3, WAV, FLAC, OGG
                            </p>
                        </>
                    )}
                </div>

                {file && (
                    <div className="space-y-6 mt-10">
                        <div className="space-y-1.5">
                            <label className="text-xs font-bold uppercase tracking-[0.2em] text-muted-foreground ml-1">
                                Track Title
                            </label>
                            <input
                                value={title}
                                onChange={(e) => setTitle(e.target.value)}
                                placeholder="e.g. Bohemian Rhapsody"
                                className="w-full px-4 py-3 bg-muted/30 border-[1.5px] border-border rounded-xl text-foreground focus:outline-none focus:ring-2 focus:ring-primary/40 transition-all"
                            />
                        </div>
                        <div className="space-y-1.5">
                            <label className="text-xs font-bold uppercase tracking-[0.2em] text-muted-foreground ml-1">
                                Artist Name
                            </label>
                            <input
                                value={artist}
                                onChange={(e) => setArtist(e.target.value)}
                                placeholder="e.g. Queen"
                                className="w-full px-4 py-3 bg-muted/30 border-[1.5px] border-border rounded-xl text-foreground focus:outline-none focus:ring-2 focus:ring-primary/40 transition-all"
                            />
                        </div>

                        <Button
                            onClick={handleSubmit}
                            disabled={uploading || !title}
                            className="w-full py-6 rounded-xl font-bold text-sm uppercase tracking-widest sketch-shadow-hover active:translate-y-[0px] active:shadow-none transition-all"
                        >
                            {uploading ? (
                                <span className="flex items-center gap-3">
                                    <SineWave className="w-5 h-5 animate-spin" />
                                    Uploading...
                                </span>
                            ) : (
                                "Commit to Library"
                            )}
                        </Button>
                    </div>
                )}
            </div>
        </div>
    );
}
