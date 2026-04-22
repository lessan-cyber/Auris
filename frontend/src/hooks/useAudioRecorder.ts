import { useState, useRef, useCallback, useEffect } from "react";

export function useAudioRecorder(maxDurationMs = 15000) {
    const [isRecording, setIsRecording] = useState(false);
    const [progress, setProgress] = useState(0);
    const [audioBlob, setAudioBlob] = useState<Blob | null>(null);
    const [error, setError] = useState<string | null>(null);

    const mediaRecorder = useRef<MediaRecorder | null>(null);
    const chunks = useRef<Blob[]>([]);
    const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
    const startTime = useRef<number>(0);
    const mediaStream = useRef<MediaStream | null>(null);

    const startRecording = useCallback(async () => {
        // Guard against concurrent starts
        if (isRecording) {
            console.warn("Recording already in progress");
            return;
        }

        try {
            // Clean up any existing resources
            stopRecording();

            const stream = await navigator.mediaDevices.getUserMedia({
                audio: true,
            });
            mediaStream.current = stream;
            
            // Detect supported MIME type before creating recorder
            const supportedTypes = [
                "audio/webm;codecs=opus",
                "audio/ogg;codecs=opus",
                "audio/webm",
                "audio/ogg",
                "audio/mp4",
                "audio/wav",
            ];
            let mimeType = "audio/wav"; // Fallback
            for (const type of supportedTypes) {
                if (MediaRecorder.isTypeSupported(type)) {
                    mimeType = type;
                    break;
                }
            }
            
            // Create recorder with the selected MIME type
            const recorder = new MediaRecorder(stream, { mimeType });
            chunks.current = [];

            recorder.ondataavailable = (e) => {
                if (e.data.size > 0) chunks.current.push(e.data);
            };

            recorder.onstop = () => {
                // Use the recorder's actual MIME type for the blob
                const finalMimeType = recorder.mimeType || mimeType;
                const blob = new Blob(chunks.current, { type: finalMimeType });
                setAudioBlob(blob);
            };

            recorder.onerror = (e) => {
                console.error("Recorder error:", e);
                setError("Recording failed due to technical error");
                stopRecording();
            };

            recorder.start(100); // Collect every 100ms
            mediaRecorder.current = recorder;
            startTime.current = Date.now();
            setIsRecording(true);
            setProgress(0);
            setError(null);
            setAudioBlob(null);

            timerRef.current = setInterval(() => {
                const elapsed = Date.now() - startTime.current;
                const pct = Math.min((elapsed / maxDurationMs) * 100, 100);
                setProgress(pct);

                if (elapsed >= maxDurationMs) {
                    stopRecording();
                }
            }, 100);
        } catch (err) {
            console.error("Microphone access error:", err);
            setError("Microphone access denied or unavailable");
            stopRecording();
        }
    }, [maxDurationMs, isRecording]);

    const stopRecording = useCallback(() => {
        // Guard against stopping when not recording
        if (!isRecording && !mediaRecorder.current) {
            return;
        }

        try {
            // Clear timer
            if (timerRef.current) {
                clearInterval(timerRef.current);
                timerRef.current = null;
            }

            // Stop recorder if active
            if (mediaRecorder.current && mediaRecorder.current.state !== "inactive") {
                mediaRecorder.current.stop();
            }

            // Stop all media stream tracks
            if (mediaStream.current) {
                mediaStream.current.getTracks().forEach((track) => {
                    try {
                        track.stop();
                    } catch (e) {
                        console.warn("Error stopping media track:", e);
                    }
                });
                mediaStream.current = null;
            }

            setIsRecording(false);
            setProgress(100);
        } catch (error) {
            console.error("Error stopping recording:", error);
            setIsRecording(false);
        }
    }, []);

    const reset = useCallback(() => {
        stopRecording(); // Ensure cleanup
        setAudioBlob(null);
        setProgress(0);
        setError(null);
        chunks.current = [];
    }, [stopRecording]);

    // Cleanup on unmount
    useEffect(() => {
        return () => {
            stopRecording();
        };
    }, [stopRecording]);

    return {
        isRecording,
        progress,
        audioBlob,
        error,
        startRecording,
        stopRecording,
        reset,
    };
}
