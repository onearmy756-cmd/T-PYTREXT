import { useCallback, useEffect, useState } from "react";
import { MapPin, Navigation, Bell, Search, Crosshair, Trash2 } from "lucide-react";
import {
  mapsService,
  type GeoPlace,
  type MapAlarm,
  type MapsStatus,
  type NearbyPlace,
  type RouteResult,
  type SavedPlace,
} from "#/api/education-service/maps.api";
import {
  EduBadge,
  EduButton,
  EduCard,
  EduEmpty,
  EduError,
  EduField,
  EduInput,
  EduSelect,
} from "./edu-ui";

/**
 * MTECHOS — Sehemu 3: Map Service (moduli 20).
 * Uelekezaji halisi (OSRM), kutafuta mahali (Nominatim), maeneo ya karibu
 * (Overpass), alarm za eneo (geofence) — zote kwa Kiswahili.
 */
export function MapsTab() {
  const [status, setStatus] = useState<MapsStatus | null>(null);
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  // Utafutaji wa mahali
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<GeoPlace[]>([]);

  // Uelekezaji
  const [fromName, setFromName] = useState("Arusha");
  const [toName, setToName] = useState("Muhimbili, Dar es Salaam");
  const [profile, setProfile] = useState("driving");
  const [route, setRoute] = useState<RouteResult | null>(null);

  // Maeneo ya karibu
  const [amenity, setAmenity] = useState("hospital");
  const [nearby, setNearby] = useState<NearbyPlace[]>([]);
  const [myLat, setMyLat] = useState("");
  const [myLon, setMyLon] = useState("");

  // Alarm
  const [alarms, setAlarms] = useState<MapAlarm[]>([]);
  const [alarmLabel, setAlarmLabel] = useState("");
  const [alarmRadius, setAlarmRadius] = useState("500");
  const [places, setPlaces] = useState<SavedPlace[]>([]);

  const refresh = useCallback(async () => {
    try {
      setStatus(await mapsService.status());
      setAlarms((await mapsService.alarms()).alarms);
      setPlaces((await mapsService.places()).places);
      setErr("");
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const doGeocode = async () => {
    if (!query.trim()) return;
    setBusy(true);
    try {
      const r = await mapsService.geocode(query);
      setResults(r.results ?? []);
      if (!r.ok) setErr(r.error ?? "hakuna matokeo");
      else setErr("");
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const doRoute = async () => {
    setBusy(true);
    try {
      const r = await mapsService.route({ from: fromName, to: toName, profile });
      setRoute(r);
      if (!r.ok) setErr(r.error ?? "njia haipatikani");
      else setErr("");
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const locate = () => {
    if (!navigator.geolocation) {
      setErr("Kivinjari hiki hakina GPS (geolocation).");
      return;
    }
    navigator.geolocation.getCurrentPosition(
      (p) => {
        setMyLat(p.coords.latitude.toFixed(6));
        setMyLon(p.coords.longitude.toFixed(6));
        setErr("");
      },
      (e) => setErr(`GPS imeshindwa: ${e.message}`),
      { enableHighAccuracy: true, timeout: 10000 },
    );
  };

  const doNearby = async () => {
    const lat = parseFloat(myLat);
    const lon = parseFloat(myLon);
    if (Number.isNaN(lat) || Number.isNaN(lon)) {
      setErr("Bofya 'Pata mahali pangu' kwanza.");
      return;
    }
    setBusy(true);
    try {
      const r = await mapsService.nearby(lat, lon, amenity);
      setNearby(r.results ?? []);
      setErr(r.ok ? "" : (r.error ?? "hakuna"));
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const addAlarm = async () => {
    if (!alarmLabel.trim()) return;
    setBusy(true);
    try {
      await mapsService.addAlarm({
        label: alarmLabel,
        query: alarmLabel,
        radius_m: parseFloat(alarmRadius) || 500,
      });
      setAlarmLabel("");
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  const checkAlarms = async () => {
    const lat = parseFloat(myLat);
    const lon = parseFloat(myLon);
    if (Number.isNaN(lat) || Number.isNaN(lon)) {
      setErr("Pata mahali pako kwanza ili kuangalia alarm.");
      return;
    }
    setBusy(true);
    try {
      const r = await mapsService.checkAlarm(lat, lon);
      setErr("");
      if (r.fired_count > 0) {
        setPlaces((p) => p);
        alert(r.note_sw);
      }
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-3" data-testid="maps-tab">
      <EduCard testId="maps-status">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="text-sm font-semibold text-content">
            Map Service — OpenStreetMap (Nominatim · OSRM · Overpass)
          </span>
          <div className="flex flex-wrap gap-1">
            <EduBadge testId="maps-places">{status?.places ?? 0} maeneo</EduBadge>
            <EduBadge testId="maps-alarms">{status?.alarms_active ?? 0} alarm hai</EduBadge>
            <EduBadge testId="maps-trips">{status?.trips ?? 0} safari</EduBadge>
          </div>
        </div>
        <p className="mt-1 text-xs text-muted">
          Huduma halisi za OpenStreetMap — hakuna API key. Bila internet, uelekezaji
          hurudi kwa umbali wa mstari ulionyooka (hautengenezi barabara).
        </p>
      </EduCard>

      {err ? <EduError message={err} /> : null}

      <EduCard testId="maps-search">
        <span className="text-sm font-semibold text-content">Tafuta mahali</span>
        <div className="mt-2 flex flex-wrap items-end gap-2">
          <div className="min-w-56 flex-1">
            <EduField label="Jina la mahali">
              <EduInput
                testId="maps-query"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Muhimbili, Kariakoo, Sokoine University…"
                onKeyDown={(e) => e.key === "Enter" && void doGeocode()}
              />
            </EduField>
          </div>
          <EduButton testId="maps-search-btn" disabled={busy} onClick={() => void doGeocode()}>
            <Search className="size-4" aria-hidden /> Tafuta
          </EduButton>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {results.length === 0 ? (
            <EduEmpty>Hakuna matokeo bado.</EduEmpty>
          ) : (
            results.map((r, i) => (
              <div
                key={`${r.lat}-${r.lon}-${i}`}
                data-testid={`maps-result-${i}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <div className="min-w-0">
                  <div className="text-sm text-content">{r.label || r.display_name}</div>
                  <div className="truncate text-xs text-muted">{r.display_name}</div>
                </div>
                <div className="flex items-center gap-2">
                  <EduBadge>
                    {r.lat.toFixed(4)}, {r.lon.toFixed(4)}
                  </EduBadge>
                  <EduButton
                    testId={`maps-result-use-${i}`}
                    onClick={() => {
                      setFromName(`${r.lat},${r.lon}`);
                      setMyLat(String(r.lat));
                      setMyLon(String(r.lon));
                    }}
                  >
                    Tumia
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="maps-route">
        <span className="text-sm font-semibold text-content">Uelekezaji (hatua kwa hatua)</span>
        <div className="mt-2 grid gap-2 md:grid-cols-4">
          <EduField label="Kutoka">
            <EduInput testId="maps-from" value={fromName} onChange={(e) => setFromName(e.target.value)} />
          </EduField>
          <EduField label="Kwenda">
            <EduInput testId="maps-to" value={toName} onChange={(e) => setToName(e.target.value)} />
          </EduField>
          <EduField label="Usafiri">
            <EduSelect testId="maps-profile" value={profile} onChange={(e) => setProfile(e.target.value)}>
              <option value="driving">Gari</option>
              <option value="walking">Kwa miguu</option>
              <option value="cycling">Baiskeli</option>
            </EduSelect>
          </EduField>
          <div className="flex items-end">
            <EduButton testId="maps-route-btn" disabled={busy} onClick={() => void doRoute()}>
              <Navigation className="size-4" aria-hidden /> Pata njia
            </EduButton>
          </div>
        </div>
        {route?.ok ? (
          <div className="mt-2 flex flex-col gap-1" data-testid="maps-route-out">
            <div className="flex flex-wrap gap-1">
              <EduBadge testId="maps-route-km">{route.distance_km} km</EduBadge>
              {route.duration_min ? <EduBadge>{route.duration_min} dakika</EduBadge> : null}
              {route.offline ? <EduBadge testId="maps-route-offline">offline</EduBadge> : null}
            </div>
            <ol className="ml-4 list-decimal text-sm text-content">
              {route.steps.map((s, i) => (
                <li key={`step-${i}`}>{s}</li>
              ))}
            </ol>
            {route.note_sw ? <p className="text-xs text-muted">{route.note_sw}</p> : null}
          </div>
        ) : null}
      </EduCard>

      <EduCard testId="maps-nearby">
        <span className="text-sm font-semibold text-content">Mahali pangu na maeneo ya karibu</span>
        <div className="mt-2 grid gap-2 md:grid-cols-4">
          <div className="flex items-end">
            <EduButton testId="maps-locate" disabled={busy} onClick={locate}>
              <Crosshair className="size-4" aria-hidden /> Pata mahali pangu
            </EduButton>
          </div>
          <EduField label="Latitude">
            <EduInput testId="maps-lat" value={myLat} onChange={(e) => setMyLat(e.target.value)} />
          </EduField>
          <EduField label="Longitude">
            <EduInput testId="maps-lon" value={myLon} onChange={(e) => setMyLon(e.target.value)} />
          </EduField>
          <EduField label="Aina ya mahali">
            <EduSelect testId="maps-amenity" value={amenity} onChange={(e) => setAmenity(e.target.value)}>
              <option value="hospital">Hospitali</option>
              <option value="pharmacy">Duka la dawa</option>
              <option value="restaurant">Hoteli / mgahawa</option>
              <option value="bank">Benki</option>
              <option value="school">Shule</option>
              <option value="fuel">Kituo cha mafuta</option>
            </EduSelect>
          </EduField>
        </div>
        <div className="mt-2 flex items-center gap-2">
          <EduButton testId="maps-nearby-btn" disabled={busy} onClick={() => void doNearby()}>
            <MapPin className="size-4" aria-hidden /> Tafuta karibu
          </EduButton>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {nearby.length === 0 ? (
            <EduEmpty>Hakuna maeneo yaliyoonyeshwa bado.</EduEmpty>
          ) : (
            nearby.map((p, i) => (
              <div
                key={`${p.lat}-${p.lon}-${i}`}
                data-testid={`maps-nearby-${i}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">{p.name}</span>
                <div className="flex gap-1">
                  <EduBadge>{p.distance_m} m</EduBadge>
                  <EduBadge>{p.direction}</EduBadge>
                  {p.phone ? <EduBadge>{p.phone}</EduBadge> : null}
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="maps-alarms">
        <span className="text-sm font-semibold text-content">Alarm za eneo ({alarms.length})</span>
        <div className="mt-2 grid gap-2 md:grid-cols-3">
          <EduField label="Mahali (jina au lat,lon)">
            <EduInput
              testId="maps-alarm-label"
              value={alarmLabel}
              onChange={(e) => setAlarmLabel(e.target.value)}
              placeholder="Hospitali ya Muhimbili"
            />
          </EduField>
          <EduField label="Eneo (mita)">
            <EduInput
              testId="maps-alarm-radius"
              value={alarmRadius}
              onChange={(e) => setAlarmRadius(e.target.value)}
            />
          </EduField>
          <div className="flex items-end gap-2">
            <EduButton testId="maps-alarm-add" disabled={busy} onClick={() => void addAlarm()}>
              <Bell className="size-4" aria-hidden /> Weka alarm
            </EduButton>
            <EduButton testId="maps-alarm-check" disabled={busy} onClick={() => void checkAlarms()}>
              Angalia
            </EduButton>
          </div>
        </div>
        <div className="mt-2 flex flex-col gap-1">
          {alarms.length === 0 ? (
            <EduEmpty>Hakuna alarm.</EduEmpty>
          ) : (
            alarms.map((a) => (
              <div
                key={a.id}
                data-testid={`maps-alarm-${a.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">{a.label || `${a.lat},${a.lon}`}</span>
                <div className="flex items-center gap-1">
                  <EduBadge>{a.radius_m} m</EduBadge>
                  <EduBadge testId={`maps-alarm-state-${a.id}`}>
                    {a.active ? "hai" : a.triggered_at ? "imeamilika" : "imezimwa"}
                  </EduBadge>
                  <EduButton testId={`maps-alarm-del-${a.id}`} onClick={() => void mapsService.removeAlarm(a.id).then(refresh)}>
                    <Trash2 className="size-4" aria-hidden />
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>

      <EduCard testId="maps-saved">
        <span className="text-sm font-semibold text-content">Maeneo yaliyohifadhiwa ({places.length})</span>
        <div className="mt-2 flex flex-col gap-1">
          {places.length === 0 ? (
            <EduEmpty>Hakuna maeneo yaliyohifadhiwa.</EduEmpty>
          ) : (
            places.map((p) => (
              <div
                key={p.id}
                data-testid={`maps-place-${p.id}`}
                className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border p-2"
              >
                <span className="text-sm text-content">{p.label || p.display_name}</span>
                <div className="flex items-center gap-1">
                  <EduBadge>
                    {p.lat.toFixed(4)}, {p.lon.toFixed(4)}
                  </EduBadge>
                  <EduButton testId={`maps-place-del-${p.id}`} onClick={() => void mapsService.removePlace(p.id).then(refresh)}>
                    <Trash2 className="size-4" aria-hidden />
                  </EduButton>
                </div>
              </div>
            ))
          )}
        </div>
      </EduCard>
    </div>
  );
}
