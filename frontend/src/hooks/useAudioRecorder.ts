import { useState, useRef, useCallback } from "react";

export function useAudioRecorder(maxDurationMs = 15000) {
    const [isRecording, setIsRecording] = useState(false);
    const [progress, setProgress] = useState(0);
    const [audioBlob, setAudioBlob] = useState<Blob | null>(null);
    const [error, setError] = useState<string | null>(null);

    const mediaRecorder = useRef<MediaRecorder | null>(null);
    const chunks = useRef<Blob[]>([]);
    const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
    const startTime = useRef<number>(0);

    const startRecording = useCallback(async () => {
        try {
            const stream = await navigator.mediaDevices.getUserMedia({
                audio: true,
            });
            const recorder = new MediaRecorder(stream);
            chunks.current = [];

            recorder.ondataavailable = (e) => {
                if (e.data.size > 0) chunks.current.push(e.data);
            };

            recorder.onstop = () => {
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
                const blob = new Blob(chunks.current, { type: mimeType });
                setAudioBlob(blob);
                stream.getTracks().forEach((t) => t.stop());
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
            setError("Microphone access denied or unavailable");
        }
    }, [maxDurationMs]);

    const stopRecording = useCallback(() => {
        if (timerRef.current) clearInterval(timerRef.current);
        mediaRecorder.current?.stop();
        setIsRecording(false);
        setProgress(100);
    }, []);

    const reset = useCallback(() => {
        setAudioBlob(null);
        setProgress(0);
        setError(null);
        chunks.current = [];
    }, []);

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
