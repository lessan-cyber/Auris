import { useEffect, useRef, useState, useCallback } from "react";
import type { JobMessage } from "@/types";

const API_URL = import.meta.env.API_URL || "http://localhost:8000";
const WS_URL = API_URL.replace(/^http/, "ws") + "/ws";

export function useTrackWebSocket() {
    const [messages, setMessages] = useState<{ track_id: string; status: string; message?: string }[]>([]);
    const [unreadCount, setUnreadCount] = useState(0);
    const ws = useRef<WebSocket | null>(null);
    const [connected, setConnected] = useState(false);

    const connect = useCallback((trackId: string) => {
        if (ws.current?.readyState === WebSocket.OPEN) return;

        const socket = new WebSocket(WS_URL);

        socket.onopen = () => {
            setConnected(true);
            socket.send(trackId);
        };

        socket.onmessage = (event) => {
            const data: JobMessage = JSON.parse(event.data);
            // Transform message to format expected by Navbar
            const notification = {
                track_id: data.track_id,
                status: data.status,
                message: data.message || `Processing: ${data.progress || 0}%`
            };
            setMessages((prev) => [notification, ...prev]);
            if (data.status === "completed" || data.status === "failed") {
                setUnreadCount((c) => c + 1);
            }
        };

        socket.onclose = () => setConnected(false);
        ws.current = socket;
    }, []);

    const disconnect = useCallback(() => {
        ws.current?.close();
        ws.current = null;
    }, []);

    const clearUnread = useCallback(() => setUnreadCount(0), []);

    useEffect(() => {
        return () => disconnect();
    }, [disconnect]);

    return {
        messages,
        unreadCount,
        connected,
        connect,
        disconnect,
        clearUnread,
    };
}
