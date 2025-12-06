import { xchacha20poly1305 } from "@noble/ciphers/chacha.js";
import { randomBytes } from "@noble/ciphers/utils.js";
import { Store } from "@tauri-apps/plugin-store";

const KEY_BYTES = 32;
const NONCE_BYTES = 24;
const STORE_PATH = "echo-secrets.json";
const STORE_KEYS = { key: "encryption_key", device: "device_id" } as const;

let store: Store | null = null;
const getStore = async () => (store ??= await Store.load(STORE_PATH));

export const generateSecretKey = () => randomBytes(KEY_BYTES);

export function encrypt(
  plaintext: string,
  key: Uint8Array
): { ciphertext: string; nonce: string } {
  const nonce = randomBytes(NONCE_BYTES);
  const cipher = xchacha20poly1305(key, nonce);
  return {
    ciphertext: toBase64(cipher.encrypt(new TextEncoder().encode(plaintext))),
    nonce: toBase64(nonce),
  };
}

export function decrypt(
  ciphertext: string,
  nonce: string,
  key: Uint8Array
): string {
  const cipher = xchacha20poly1305(key, fromBase64(nonce));
  return new TextDecoder().decode(cipher.decrypt(fromBase64(ciphertext)));
}

export async function getKeyFingerprint(key: Uint8Array): Promise<string> {
  const hash = await crypto.subtle.digest(
    "SHA-256",
    new Uint8Array(key).buffer as ArrayBuffer
  );
  return toHex(new Uint8Array(hash)).slice(0, 8).toUpperCase();
}

export function generateLinkUri(
  deviceId: string,
  key: Uint8Array,
  wsUrl: string
): string {
  return `echo://connect?${new URLSearchParams({
    id: deviceId,
    key: toBase64Url(key),
    server: wsUrl,
  })}`;
}

export async function saveEncryptionKey(key: Uint8Array): Promise<void> {
  const s = await getStore();
  await s.set(STORE_KEYS.key, toBase64Url(key));
  await s.save();
}

export async function loadEncryptionKey(): Promise<Uint8Array | null> {
  try {
    const keyB64 = await (await getStore()).get<string>(STORE_KEYS.key);
    return keyB64 ? fromBase64Url(keyB64) : null;
  } catch {
    return null;
  }
}

export async function clearEncryptionKey(): Promise<void> {
  const s = await getStore();
  await s.delete(STORE_KEYS.key);
  await s.save();
}

export async function getOrCreateDeviceId(): Promise<string> {
  try {
    const s = await getStore();
    let id = await s.get<string>(STORE_KEYS.device);
    if (!id) {
      id = crypto.randomUUID();
      await s.set(STORE_KEYS.device, id);
      await s.save();
    }
    return id;
  } catch {
    return crypto.randomUUID();
  }
}

const toBase64 = (b: Uint8Array) => btoa(String.fromCharCode(...b));
const fromBase64 = (s: string) =>
  Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
const toBase64Url = (b: Uint8Array) =>
  toBase64(b).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
const fromBase64Url = (s: string) =>
  fromBase64(
    s
      .replace(/-/g, "+")
      .replace(/_/g, "/")
      .padEnd(s.length + ((4 - (s.length % 4)) % 4), "=")
  );
const toHex = (b: Uint8Array) =>
  [...b].map((x) => x.toString(16).padStart(2, "0")).join("");
