import { useEffect } from "react";
import { Microphone, MicrophoneMute } from "iconoir-react";
import { Button } from "@/components/ui/button";
import { useAudioRecorder } from "@/hooks/useAudioRecorder";

interface AudioRecorderProps {
    onRecorded: (blob: Blob) => void;
}

export function AudioRecorder({ onRecorded }: AudioRecorderProps) {
    const {
        isRecording,
        progress,
        audioBlob,
        startRecording,
        stopRecording,
        reset,
    } = useAudioRecorder(15000);

    useEffect(() => {
        if (audioBlob) {
            onRecorded(audioBlob);
        }
    }, [audioBlob, onRecorded]);

    return (
        <div className="flex flex-col items-center gap-8 py-12">
            <div className="relative">
                {/* Recording button */}
                <button
                    onClick={isRecording ? stopRecording : startRecording}
                    className={`w-24 h-24 rounded-full flex items-center justify-center transition-all ${
                        isRecording
                            ? "bg-red-500 hover:bg-red-600 animate-pulse"
                            : "bg-neutral-900 hover:bg-neutral-800"
                    }`}
                >
                    {isRecording ? (
                        <MicrophoneMute className="w-8 h-8 text-white" />
                    ) : (
                        <Microphone className="w-8 h-8 text-white" />
                    )}
                </button>

                {/* Progress ring */}
                {isRecording && (
                    <svg className="absolute -inset-2 w-28 h-28 -rotate-90">
                        <circle
                            cx="56"
                            cy="56"
                            r="52"
                            fill="none"
                            stroke="#e5e5e5"
                            strokeWidth="4"
                        />
                        <circle
                            cx="56"
                            cy="56"
                            r="52"
                            fill="none"
                            stroke="#171717"
                            strokeWidth="4"
                            strokeDasharray={`${progress * 3.27} 327`}
                            className="transition-all duration-100"
                        />
                    </svg>
                )}
            </div>

            <div className="text-center space-y-2">
                <p className="text-lg font-medium text-neutral-900">
                    {isRecording
                        ? "Recording..."
                        : audioBlob
                          ? "Recording complete"
                          : "Tap to record"}
                </p>
                <p className="text-sm text-neutral-500">
                    {isRecording
                        ? `${Math.ceil((100 - progress) * 0.15)}s remaining`
                        : "Record up to 15 seconds of audio"}
                </p>
            </div>

            {/* Fake waveform visualization */}
            {isRecording && (
                <div className="flex items-center gap-1 h-16">
                    {Array.from({ length: 40 }).map((_, i) => (
                        <div
                            key={i}
                            className="w-1.5 bg-neutral-900 rounded-full animate-pulse"
                            style={{
                                height: `${Math.random() * 100}%`,
                                animationDelay: `${i * 0.05}s`,
                            }}
                        />
                    ))}
                </div>
            )}

            {audioBlob && (
                <div className="flex gap-2">
                    <Button variant="outline" onClick={reset}>
                        Record again
                    </Button>
                </div>
            )}
        </div>
    );
}
