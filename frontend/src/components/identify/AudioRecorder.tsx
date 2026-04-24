import { useState, useRef, useEffect } from "react";
import { Microphone, MicrophoneMute } from "iconoir-react";
import { Button } from "@/components/ui/button";
import { useAudioRecorder } from "@/hooks/useAudioRecorder";

interface AudioRecorderProps {
    onRecorded: (blob: Blob) => void;
}

export function AudioRecorder({ onRecorded }: AudioRecorderProps) {
    const {
        isRecording,
        isStarting,
        progress,
        audioBlob,
        analyser,
        startRecording,
        stopRecording,
        reset,
    } = useAudioRecorder(15000);

    const [waveform, setWaveform] = useState<number[]>(Array(40).fill(5));
    const animationRef = useRef<number | null>(null);
    const waveformBars = isRecording ? waveform : Array(40).fill(5);

    useEffect(() => {
        if (!isRecording) {
            if (animationRef.current) {
                cancelAnimationFrame(animationRef.current);
            }
            return;
        }

        if (!analyser) return;

        const dataArray = new Uint8Array(analyser.frequencyBinCount);

        const updateWaveform = () => {
            analyser.getByteFrequencyData(dataArray);

            // Average down to 40 bars
            const step = Math.floor(dataArray.length / 40);
            const newWaveform = [];
            for (let i = 0; i < 40; i++) {
                let sum = 0;
                for (let j = 0; j < step; j++) {
                    sum += dataArray[i * step + j];
                }
                const avg = sum / step;
                newWaveform.push(Math.max(5, (avg / 255) * 100));
            }

            setWaveform(newWaveform);
            animationRef.current = requestAnimationFrame(updateWaveform);
        };

        animationRef.current = requestAnimationFrame(updateWaveform);

        return () => {
            if (animationRef.current) {
                cancelAnimationFrame(animationRef.current);
            }
        };
    }, [isRecording, analyser]);

    useEffect(() => {
        // Only trigger onRecorded if we have a blob AND we are not currently recording AND not starting
        if (audioBlob && !isRecording && !isStarting && audioBlob.size > 0) {
            onRecorded(audioBlob);
        }
    }, [audioBlob, isRecording, isStarting, onRecorded]);

    return (
        <div className="flex flex-col items-center gap-8 py-12">
            <div className="relative">
                {/* Recording button */}{" "}
                <button
                    type="button"
                    disabled={isStarting}
                    aria-label={
                        isRecording
                            ? "Stop recording"
                            : isStarting
                              ? "Starting..."
                              : "Start recording"
                    }
                    onClick={isRecording ? stopRecording : startRecording}
                    className={`w-24 h-24 rounded-full flex items-center justify-center transition-all ${
                        isRecording
                            ? "bg-red-500 hover:bg-red-600 animate-pulse shadow-[0_0_20px_rgba(239,68,68,0.4)]"
                            : isStarting
                              ? "bg-neutral-500 cursor-wait"
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
                    <svg className="pointer-events-none absolute -inset-2 w-28 h-28 -rotate-90">
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
                        : isStarting
                          ? "Requesting microphone..."
                          : audioBlob
                            ? "Recording complete"
                            : "Tap to record"}
                </p>
                <p className="text-sm text-neutral-500">
                    {isRecording
                        ? `${Math.ceil((100 - progress) * 0.15)}s remaining`
                        : isStarting
                          ? "Please allow access"
                          : "Record up to 15 seconds of audio"}
                </p>
            </div>

            {/* Real waveform visualization */}
            <div className="flex items-center gap-1 h-16 w-full max-w-60 justify-center">
                {waveformBars.map((height, i) => (
                    <div
                        key={i}
                        className={`w-1 rounded-full transition-all duration-75 ${
                            isRecording ? "bg-primary" : "bg-neutral-200"
                        }`}
                        style={{
                            height: `${height}%`,
                        }}
                    />
                ))}
            </div>

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
