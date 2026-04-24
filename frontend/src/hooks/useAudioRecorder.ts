import { useState, useRef, useCallback, useEffect } from "react";

function encodeAudioBufferToWav(buffer: AudioBuffer): Blob {
    const numChannels = buffer.numberOfChannels;
    const sampleRate = buffer.sampleRate;
    const length = buffer.length;
    const bytesPerSample = 2;
    const blockAlign = numChannels * bytesPerSample;
    const dataLength = length * blockAlign;
    const arrayBuffer = new ArrayBuffer(44 + dataLength);
    const view = new DataView(arrayBuffer);

    const writeString = (offset: number, value: string) => {
        for (let i = 0; i < value.length; i++) {
            view.setUint8(offset + i, value.charCodeAt(i));
        }
    };

    let offset = 0;
    writeString(offset, "RIFF");
    offset += 4;
    view.setUint32(offset, 36 + dataLength, true);
    offset += 4;
    writeString(offset, "WAVE");
    offset += 4;
    writeString(offset, "fmt ");
    offset += 4;
    view.setUint32(offset, 16, true);
    offset += 4;
    view.setUint16(offset, 1, true);
    offset += 2;
    view.setUint16(offset, numChannels, true);
    offset += 2;
    view.setUint32(offset, sampleRate, true);
    offset += 4;
    view.setUint32(offset, sampleRate * blockAlign, true);
    offset += 4;
    view.setUint16(offset, blockAlign, true);
    offset += 2;
    view.setUint16(offset, 16, true);
    offset += 2;
    writeString(offset, "data");
    offset += 4;
    view.setUint32(offset, dataLength, true);
    offset += 4;

    const channelData = Array.from({ length: numChannels }, (_, channel) =>
        buffer.getChannelData(channel),
    );

    let sampleOffset = 44;
    for (let i = 0; i < length; i++) {
        for (let channel = 0; channel < numChannels; channel++) {
            const sample = Math.max(-1, Math.min(1, channelData[channel][i]));
            view.setInt16(sampleOffset, sample < 0 ? sample * 0x8000 : sample * 0x7fff, true);
            sampleOffset += 2;
        }
    }

    return new Blob([arrayBuffer], { type: "audio/wav" });
}

export function useAudioRecorder(maxDurationMs = 15000) {
    const [isRecording, setIsRecording] = useState(false);
    const [isStarting, setIsStarting] = useState(false);
    const [progress, setProgress] = useState(0);
    const [audioBlob, setAudioBlob] = useState<Blob | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [analyser, setAnalyser] = useState<AnalyserNode | null>(null);

    const mediaRecorder = useRef<MediaRecorder | null>(null);
    const chunks = useRef<Blob[]>([]);
    const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
    const startTime = useRef<number>(0);
    const mediaStream = useRef<MediaStream | null>(null);
    const audioContext = useRef<AudioContext | null>(null);
    const recordingSession = useRef(0);

    const stopRecording = useCallback(() => {
        try {
            if (timerRef.current) {
                clearInterval(timerRef.current);
                timerRef.current = null;
            }

            if (mediaRecorder.current && mediaRecorder.current.state !== "inactive") {
                mediaRecorder.current.stop();
            }

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
            setIsStarting(false);
        } catch (err) {
            console.error("Error stopping recording:", err);
            setIsRecording(false);
            setIsStarting(false);
        }
    }, []);

    const startRecording = useCallback(async () => {
        if (isRecording || isStarting) {
            return;
        }

        setIsStarting(true);
        setAudioBlob(null);
        setError(null);
        setProgress(0);
        recordingSession.current += 1;
        const sessionId = recordingSession.current;

        try {
            const stream = await navigator.mediaDevices.getUserMedia({
                audio: true,
            });
            
            mediaStream.current = stream;
            
            audioContext.current = new AudioContext();
            const recorderContext = audioContext.current;
            const source = recorderContext.createMediaStreamSource(stream);
            const newAnalyser = recorderContext.createAnalyser();
            newAnalyser.fftSize = 256;
            source.connect(newAnalyser);
            setAnalyser(newAnalyser);

            // Prioritize compressed formats for better performance
            const supportedTypes = [
                "audio/ogg;codecs=opus",
                "audio/webm;codecs=opus",
                "audio/ogg",
                "audio/webm",
                "audio/mp4",
                "audio/wav",
            ];
            let mimeType = "audio/wav";
            for (const type of supportedTypes) {
                if (MediaRecorder.isTypeSupported(type)) {
                    mimeType = type;
                    break;
                }
            }
            
            const recorder = new MediaRecorder(stream, { mimeType });
            chunks.current = [];

            recorder.ondataavailable = (e) => {
                if (e.data.size > 0) chunks.current.push(e.data);
            };

            recorder.onstop = async () => {
                const finalMimeType = recorder.mimeType || mimeType;
                const recordedBlob = new Blob(chunks.current, { type: finalMimeType });
                const decodeContext = recorderContext;
                const isCurrentSession = recordingSession.current === sessionId;

                try {
                    if (!isCurrentSession) {
                        return;
                    }

                    if (recordedBlob.size > 0) {
                        if (!decodeContext || decodeContext.state === "closed") {
                            throw new Error("Audio context is not available");
                        }

                        const audioBuffer = await decodeContext.decodeAudioData(
                            await recordedBlob.arrayBuffer(),
                        );
                        const wavBlob = encodeAudioBufferToWav(audioBuffer);
                        setAudioBlob(wavBlob);
                    }
                } catch (err) {
                    if (!isCurrentSession) {
                        return;
                    }
                    console.error("Failed to normalize recording:", err);
                    setError("Recording finished, but the audio format could not be prepared for upload");
                } finally {
                    if (decodeContext && decodeContext.state !== "closed") {
                        await decodeContext.close();
                    }

                    if (audioContext.current === decodeContext) {
                        audioContext.current = null;
                        setAnalyser(null);
                    }
                }
            };

            recorder.onerror = (e) => {
                console.error("Recorder error:", e);
                setError("Recording failed due to technical error");
                stopRecording();
            };

            recorder.start(100);
            mediaRecorder.current = recorder;
            startTime.current = Date.now();
            setIsRecording(true);
            setIsStarting(false);

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
    }, [maxDurationMs, isRecording, isStarting, stopRecording]);

    const reset = useCallback(() => {
        stopRecording();
        setAudioBlob(null);
        setProgress(0);
        setError(null);
        chunks.current = [];
    }, [stopRecording]);

    useEffect(() => {
        return () => {
            stopRecording();
        };
    }, [stopRecording]);

    return {
        isRecording,
        isStarting,
        progress,
        audioBlob,
        error,
        analyser,
        startRecording,
        stopRecording,
        reset,
    };
}
