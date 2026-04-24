import { createContext } from "react";

export interface UploadNotificationContextType {
    uploadNotifications: { track_id: string; status: string; message?: string }[];
    setUploadNotifications: React.Dispatch<React.SetStateAction<{ track_id: string; status: string; message?: string }[]>>;
}

export const UploadNotificationContext = createContext<UploadNotificationContextType | undefined>(undefined);
