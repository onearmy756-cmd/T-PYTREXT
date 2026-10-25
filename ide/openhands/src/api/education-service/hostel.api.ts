/**
 * Hosteli / malazi — ELIMUPRO Awamu 10.
 * Hupiga `/api/hostel/*` kwenye fundi-deploy (server/agent/src/hostel.rs).
 */

import {
  getAgentServerBaseUrl,
  getAgentServerHeaders,
} from "#/api/agent-server-config";

async function hostelCall<T>(path: string, body?: unknown): Promise<T> {
  const base = getAgentServerBaseUrl();
  if (!base) throw new Error("Backend base URL haijulikani");
  const url = `${base.replace(/\/+$/, "")}/api/hostel${path}`;
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

export interface HostelStatus {
  ok: boolean;
  hostels: number;
  rooms: number;
  beds: number;
  occupied: number;
  free: number;
  occupancy_pct: number;
  allocations: number;
}

export interface Hostel {
  id: string;
  name: string;
  gender: string;
  warden: string;
  rooms: number;
  beds: number;
  occupied: number;
  free: number;
}

export interface HostelBed {
  id: string;
  label: string;
  status: string;
  student: string | null;
  student_id: string | null;
}

export interface HostelRoom {
  id: string;
  room_no: string;
  beds: number;
  occupied: number;
  beds_detail: HostelBed[];
}

export interface HostelAllocation {
  id: string;
  student: string;
  reg_no: string;
  hostel: string;
  room_no: string;
  bed: string;
  active: boolean;
  vacated_at: string | null;
}

export interface Occupancy {
  hostel_id: string;
  hostel: string;
  gender: string;
  beds: number;
  occupied: number;
  free: number;
  occupancy_pct: number;
}

export const hostelService = {
  status: () => hostelCall<HostelStatus>("/status"),
  hostels: () => hostelCall<{ ok: boolean; hostels: Hostel[] }>("/list"),
  addHostel: (body: { name: string; gender?: string; warden?: string }) =>
    hostelCall<{ ok: boolean; id?: string; gender?: string; error?: string }>("/add", body),
  hostel: (id: string) =>
    hostelCall<{ ok: boolean; hostel: { id: string; name: string; gender: string }; rooms: HostelRoom[] }>(
      `/${id}`,
    ),
  addRoom: (body: { hostel_id: string; room_no: string; beds?: number }) =>
    hostelCall<{ ok: boolean; id?: string; beds?: number; error?: string }>("/room/add", body),
  allocate: (body: { student_id: string; hostel_id?: string; room_id?: string; bed_id?: string }) =>
    hostelCall<{ ok: boolean; allocation_id?: string; bed_id?: string; error?: string; hostel_gender?: string }>(
      "/allocate",
      body,
    ),
  vacate: (body: { allocation_id?: string; student_id?: string }) =>
    hostelCall<{ ok: boolean; bed_id?: string; error?: string }>("/vacate", body),
  allocations: (p: { hostel_id?: string; active?: boolean }) => {
    const qs = new URLSearchParams();
    if (p.hostel_id) qs.set("hostel_id", p.hostel_id);
    if (p.active === false) qs.set("active", "false");
    return hostelCall<{ ok: boolean; allocations: HostelAllocation[] }>(`/allocations?${qs}`);
  },
  occupancy: () => hostelCall<{ ok: boolean; occupancy: Occupancy[] }>("/occupancy"),
};
