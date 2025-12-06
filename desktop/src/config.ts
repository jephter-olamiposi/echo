const API_URL = import.meta.env.VITE_API_URL || "http://localhost:3000";
const WS_URL = import.meta.env.VITE_WS_URL || "ws://localhost:3000";

export const config = {
  apiUrl: API_URL,
  wsUrl: WS_URL,
  reconnectMs: 3000,
  previewLen: 50,
} as const;

const TOKEN_KEY = "echo_token";
export const saveToken = (t: string) => localStorage.setItem(TOKEN_KEY, t);
export const loadToken = () => localStorage.getItem(TOKEN_KEY);
export const clearToken = () => localStorage.removeItem(TOKEN_KEY);
