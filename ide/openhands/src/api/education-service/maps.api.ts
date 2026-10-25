/**
 * Map Service — MTECHOS Sehemu 3 (moduli 20).
 * Hupiga `/api/maps/*` (server/agent/src/maps.rs) — Nominatim, OSRM, Overpass.
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function mapCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/maps${path}`;
  const res = await fetch(url, {
    method: body !== undefined ? "POST" : "GET",
    headers: {
      ...getAgentServerHeaders(),
      ...(body !== undefined ? { "Content-Type": "application/json" } : {}),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
  const data = (await res.json().catch(() => ({}))) as T & { error?: string };
  if (!res.ok) throw new Error(data?.error || `HTTP ${res.status}`);
  return data;
}

export interface MapsStatus {
  ok: boolean;
  module: string;
  part: string;
  providers: { geocode: string; routing: string; poi: string };
  places: number;
  alarms: number;
  alarms_active: number;
  trips: number;
}

export interface GeoPlace {
  label: string;
  display_name: string;
  lat: number;
  lon: number;
  type: string;
  category: string;
}

export interface RouteResult {
  ok: boolean;
  offline?: boolean;
  distance_m: number;
  distance_km: number;
  duration_s?: number;
  duration_min?: number;
  steps: string[];
  bearing_deg?: number;
  direction?: string;
  source?: string;
  note_sw?: string;
  error?: string;
}

export interface NearbyPlace {
  name: string;
  amenity: string;
  lat: number;
  lon: number;
  distance_m: number;
  direction: string;
  phone: string;
}

export interface MapAlarm {
  id: string;
  label: string;
  lat: number;
  lon: number;
  radius_m: number;
  active: boolean;
  triggered_at: string | null;
  created_at: string;
}

export interface SavedPlace {
  id: string;
  label: string;
  query: string;
  lat: number;
  lon: number;
  display_name: string;
  created_at: string;
}

export interface Trip {
  id: string;
  origin: string;
  destination: string;
  distance_m: number;
  duration_s: number;
  steps: number;
  offline: boolean;
  created_at: string;
}

export const mapsService = {
  status: () => mapCall<MapsStatus>("/status"),
  geocode: (q: string, limit = 5) =>
    mapCall<{ ok: boolean; count: number; results: GeoPlace[]; error?: string }>(
      `/geocode?q=${encodeURIComponent(q)}&limit=${limit}`,
    ),
  reverse: (lat: number, lon: number) =>
    mapCall<{ ok: boolean; display_name: string; error?: string }>(
      `/reverse?lat=${lat}&lon=${lon}`,
    ),
  route: (p: {
    from_lat?: number;
    from_lon?: number;
    to_lat?: number;
    to_lon?: number;
    from?: string;
    to?: string;
    profile?: string;
  }) => {
    const qs = new URLSearchParams();
    Object.entries(p).forEach(([k, v]) => {
      if (v !== undefined && v !== null && v !== "") qs.set(k, String(v));
    });
    return mapCall<RouteResult>(`/route?${qs.toString()}`);
  },
  nearby: (lat: number, lon: number, amenity = "hospital", radius = 2000) =>
    mapCall<{ ok: boolean; count: number; results: NearbyPlace[]; error?: string }>(
      `/nearby?lat=${lat}&lon=${lon}&amenity=${encodeURIComponent(amenity)}&radius=${radius}`,
    ),
  places: () => mapCall<{ ok: boolean; places: SavedPlace[] }>("/places"),
  addPlace: (body: { label?: string; query?: string; lat?: number; lon?: number }) =>
    mapCall<{ ok: boolean; id?: string; lat?: number; lon?: number; error?: string }>(
      "/place/add",
      body,
    ),
  removePlace: (id: string) => mapCall<{ ok: boolean }>(`/place/${id}/remove`, {}),
  alarms: () => mapCall<{ ok: boolean; alarms: MapAlarm[] }>("/alarms"),
  addAlarm: (body: { label?: string; lat?: number; lon?: number; radius_m?: number; query?: string }) =>
    mapCall<{ ok: boolean; id?: string; error?: string }>("/alarm/add", body),
  removeAlarm: (id: string) => mapCall<{ ok: boolean }>(`/alarm/${id}/remove`, {}),
  checkAlarm: (lat: number, lon: number) =>
    mapCall<{
      ok: boolean;
      fired: { id: string; label: string; distance_m: number }[];
      fired_count: number;
      nearest_label: string;
      nearest_m: number | null;
      note_sw: string;
    }>("/alarm/check", { lat, lon }),
  trips: () => mapCall<{ ok: boolean; trips: Trip[] }>("/trips"),
};
