import { useState, useEffect, createContext } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { Library } from "@/pages/Library";
import { Upload } from "@/pages/Upload";
import { Identify } from "@/pages/Identify";
import { Navbar } from "@/components/layout/Navbar";
import { trackApi } from "@/lib/api";
import { WarningTriangle, Refresh, SineWave, Database, Cloud } from "iconoir-react";
import { Button } from "@/components/ui/button";
import axios from "axios";
import { Toaster } from "@/components/ui/sonner";

// Create context for upload notifications
export interface UploadNotificationContextType {
    uploadNotifications: { track_id: string; status: string; message?: string }[];
    setUploadNotifications: React.Dispatch<React.SetStateAction<{ track_id: string; status: string; message?: string }[]>>;
}

export const UploadNotificationContext = createContext<UploadNotificationContextType | undefined>(undefined);

const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            retry: false,
        },
    },
});

function AppContent() {
    const [uploadNotifications, setUploadNotifications] = useState<{ track_id: string; status: string; message?: string }[]>([]);
    const [theme, setTheme] = useState<"light" | "dark">(
        () => (localStorage.getItem("theme") as "light" | "dark") || "dark"
    );

    const uploadNotificationContextValue = {
        uploadNotifications,
        setUploadNotifications
    };

    const { 
        data: health, 
        isLoading: isCheckingHealth, 
        isError: isBackendDown,
        error: healthError,
        refetch: retryHealthCheck 
    } = useQuery({
        queryKey: ["health"],
        queryFn: trackApi.checkHealth,
    });

    useEffect(() => {
        if (isBackendDown) {
            console.error("Backend health check failed:", healthError);
        }
        if (health && health.status !== "ok") {
            console.warn("Backend is unhealthy:", health);
        }
    }, [isBackendDown, healthError, health]);

    useEffect(() => {
        const root = window.document.documentElement;
        root.classList.remove("light", "dark");
        root.classList.add(theme);
        localStorage.setItem("theme", theme);
    }, [theme]);

    const toggleTheme = () => setTheme(prev => prev === "light" ? "dark" : "light");

    if (isCheckingHealth) {
        return (
            <div className="min-h-screen bg-background flex flex-col items-center justify-center gap-4">
                <SineWave className="w-12 h-12 text-primary animate-pulse" />
                <p className="text-muted-foreground animate-pulse font-medium">Connecting to Auris...</p>
            </div>
        );
    }

    // Extract response data and determine display health
    const responseData = axios.isAxiosError(healthError) ? healthError.response?.data as any : null;
    const displayHealth = health || responseData;
    
    // Determine error states
    const isUnhealthy = Boolean(displayHealth && displayHealth.status !== "ok");
    const isConnectionError = isBackendDown && (!axios.isAxiosError(healthError) || !healthError.response);

    if (isConnectionError || isUnhealthy) {

        return (
            <div className="min-h-screen bg-background flex flex-col items-center justify-center p-6 text-center">
                <div className="w-20 h-20 rounded-3xl bg-destructive/10 flex items-center justify-center mb-6">
                    <WarningTriangle className="w-10 h-10 text-destructive" />
                </div>
                <h1 className="text-2xl font-semibold text-foreground mb-2">
                    {isUnhealthy ? "System Unhealthy" : "Backend Unreachable"}
                </h1>
                <p className="text-muted-foreground max-w-sm mb-4">
                    {isUnhealthy 
                        ? "The server is running but some services are currently unavailable."
                        : "We couldn't connect to the Auris backend. Please ensure the server is running at http://localhost:8000."}
                </p>

                {isBackendDown && (
                    <div className="bg-red-500/10 text-red-500 text-[10px] font-mono p-2 rounded-lg mb-8 max-w-sm overflow-auto">
                        {healthError instanceof Error ? healthError.message : String(healthError)}
                    </div>
                )}

                {displayHealth && (
                    <div className="grid grid-cols-2 gap-4 mb-8 w-full max-w-xs mx-auto">
                        <div className={`p-3 rounded-xl border ${displayHealth.database === "up" ? "border-emerald-500/20 bg-emerald-500/5" : "border-red-500/20 bg-red-500/5"}`}>
                            <Database className={`w-5 h-5 mx-auto mb-1 ${displayHealth.database === "up" ? "text-emerald-500" : "text-red-500"}`} />
                            <p className="text-[10px] uppercase tracking-wider font-bold opacity-50">Database</p>
                            <p className={`text-xs font-semibold ${displayHealth.database === "up" ? "text-emerald-500" : "text-red-500"}`}>
                                {displayHealth.database === "up" ? "ONLINE" : "OFFLINE"}
                            </p>
                        </div>
                        <div className={`p-3 rounded-xl border ${displayHealth.s3 === "up" ? "border-emerald-500/20 bg-emerald-500/5" : "border-red-500/20 bg-red-500/5"}`}>
                            <Cloud className={`w-5 h-5 mx-auto mb-1 ${displayHealth.s3 === "up" ? "text-emerald-500" : "text-red-500"}`} />
                            <p className="text-[10px] uppercase tracking-wider font-bold opacity-50">Storage (S3)</p>
                            <p className={`text-xs font-semibold ${displayHealth.s3 === "up" ? "text-emerald-500" : "text-red-500"}`}>
                                {displayHealth.s3 === "up" ? "ONLINE" : "OFFLINE"}
                            </p>
                        </div>
                    </div>
                )}

                <Button 
                    onClick={() => retryHealthCheck()}
                    variant="outline"
                    className="gap-2"
                >
                    <Refresh className="w-4 h-4" />
                    Retry Connection
                </Button>
            </div>
        );
    }

    return (
        <UploadNotificationContext.Provider value={uploadNotificationContextValue}>
            <div className="min-h-screen bg-background transition-colors duration-300">
                <Navbar
                    unreadCount={uploadNotifications.filter(n => n.status === "ready" || n.status === "error").length}
                    notifications={[...uploadNotifications]}
                    onClear={() => {
                        setUploadNotifications([]);
                    }}
                    theme={theme}
                    onToggleTheme={toggleTheme}
                />
                <Routes>
                    <Route path="/" element={<Library />} />
                    <Route path="/upload" element={<Upload />} />
                    <Route path="/identify" element={<Identify />} />
                </Routes>
            </div>
        </UploadNotificationContext.Provider>
    );
}

export default function App() {
    return (
        <QueryClientProvider client={queryClient}>
            <BrowserRouter>
                <AppContent />
                <Toaster />
            </BrowserRouter>
        </QueryClientProvider>
    );
}
