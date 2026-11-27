// Types and helpers for data.json, which `orc dashboard` builds in an Action.
// Everything in it was written by contributors' agents: React escapes text,
// and URLs go through safeUrl() before use.
import { useEffect, useRef, useState } from 'react'

export type Hw = { os: string; gpu_vendors: string[]; summary: string }
export type Evidence = { type: string; ref: string }
export type Reproduction = { github: string; result: 'reproduced' | 'not-reproduced'; date: string; env?: string | null; notes?: string | null }
export type Dispute = { github: string; reason: string; date: string; env?: string | null }
export type Status = 'proposed' | 'reproduced' | 'verified' | 'disputed' | 'retracted'

export type Finding = {
  id: string; repo: string; scope: string; subject: string; kind: string; claim: string; status: Status
  created: string; author: string; agent?: string | null; model?: string | null; env?: string | null
  attempt?: number | null; depends_on: string[]; evidence: Evidence[]; suspect: boolean
  reproductions: Reproduction[]; disputes: Dispute[]; retracted_reason?: string | null; body: string; url?: string | null
}

export type Attempt = {
  repo: string; number: number; url: string; title: string; author: string; scope: string
  state: 'active' | 'paused' | 'expired'; lease_expires: string; lease_hours?: number; hw?: Hw | null
  comments: number; created_at: string; updated_at: string; hypothesis?: string | null; approach?: string | null; problem_text?: string | null
}

export type ClosedAttempt = { repo: string; number: number; url: string; title: string; author: string; scope: string; hw?: Hw | null; closed_at: string; reason: string; created_at: string }
export type Problem = { repo: string; number: number; url: string; title: string; state: string; author: string; created_at: string; sub_issues?: { total: number; completed: number } | null }
export type Game = {
  name: string; title?: string | null; platform: string; id: string; status: string; region?: string; year?: number
  repos?: string[]; cover?: string | null; targets?: string[]; tags?: string[]; description?: string
  scope: string; repo: string; added?: string | null
}
export type GameRequest = {
  repo: string; number: number; url: string; title: string; author: string; created_at: string; comments: number
  game: Game | null; errors: string[]; name?: string | null; console?: string | null
}
export type ActivityEvent = { time: string; type: string; repo: string; who: string; text: string; target: { finding?: string; attempt?: number; url?: string; reason?: string } }
export type Project = { repo: string; name: string; url?: string | null; findings: number; games: number }

export type Data = {
  generated_at: string; hub?: string | null; projects: Project[]; games: Game[]; findings: Finding[]
  attempts: Attempt[]; closed_attempts: ClosedAttempt[]; problems: Problem[]; requests?: GameRequest[]; envs: Record<string, Hw>
  activity: ActivityEvent[]; warnings: string[]
}

const POLL_MS = 30_000

export const eventKey = (e: ActivityEvent) => `${e.type}|${e.time}|${e.who}|${JSON.stringify(e.target)}`

/** Polls data.json; returns the data, which events are new since the last poll, and load state. */
export function useDashboard() {
  const [data, setData] = useState<Data | null>(null)
  const [fresh, setFresh] = useState<Set<string>>(new Set())
  const [error, setError] = useState(false)
  const seen = useRef<Set<string> | null>(null)

  useEffect(() => {
    let stop = false
    const load = async () => {
      try {
        const r = await fetch(`data.json?t=${Date.now()}`, { cache: 'no-store' })
        if (!r.ok) throw new Error(String(r.status))
        const d: Data = await r.json()
        if (stop) return
        const keys = new Set(d.activity.map(eventKey))
        setFresh(seen.current ? new Set([...keys].filter((k) => !seen.current!.has(k))) : new Set())
        seen.current = keys
        setData((old) => (old && old.generated_at === d.generated_at ? old : d))
        setError(false)
      } catch {
        if (!stop) setError(true)
      }
    }
    load()
    const t = setInterval(load, POLL_MS)
    const vis = () => !document.hidden && load()
    document.addEventListener('visibilitychange', vis)
    return () => { stop = true; clearInterval(t); document.removeEventListener('visibilitychange', vis) }
  }, [])
  return { data, fresh, error }
}

/** Re-render every `ms` so relative times stay current. */
export function useTick(ms = 15_000) {
  const [, set] = useState(0)
  useEffect(() => { const t = setInterval(() => set((n) => n + 1), ms); return () => clearInterval(t) }, [ms])
}

export function safeUrl(u: unknown): string | null {
  if (typeof u !== 'string') return null
  if (/^https:\/\/[^\s"'<>\\]+$/.test(u)) return u
  if (/^covers\/[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(u) && !u.includes('..')) return u
  return null
}

/** The hub repo (owner/name) hosting the dashboard, with a safe default. */
export const hubOf = (d: Data | null) => (d?.hub && /^[\w.-]+\/[\w.-]+$/.test(d.hub) ? d.hub : 'leonardochappuis/open-recomp')

export const isLogin = (s: string) => /^[A-Za-z0-9-]{1,39}$/.test(s)

export function ago(iso: string): string {
  const t = Date.parse(iso)
  if (!t) return ''
  const s = Math.round((Date.now() - t) / 1000)
  const a = Math.abs(s)
  const v = a < 45 ? 'just now' : a < 3600 ? `${Math.round(a / 60)}m` : a < 86400 ? `${Math.round(a / 3600)}h` : `${Math.round(a / 86400)}d`
  if (v === 'just now') return v
  return s < 0 ? `in ${v}` : `${v} ago`
}

export const PLATFORMS: Record<string, { name: string; short: string; hue: number }> = {
  n64: { name: 'Nintendo 64', short: 'N64', hue: 25 }, gc: { name: 'GameCube', short: 'GC', hue: 285 },
  wii: { name: 'Wii', short: 'Wii', hue: 220 }, wiiu: { name: 'Wii U', short: 'Wii U', hue: 210 },
  switch: { name: 'Nintendo Switch', short: 'Switch', hue: 15 }, gba: { name: 'Game Boy Advance', short: 'GBA', hue: 265 },
  nds: { name: 'Nintendo DS', short: 'DS', hue: 240 }, snes: { name: 'Super Nintendo', short: 'SNES', hue: 275 },
  ps1: { name: 'PlayStation', short: 'PS1', hue: 230 }, ps2: { name: 'PlayStation 2', short: 'PS2', hue: 250 },
  ps3: { name: 'PlayStation 3', short: 'PS3', hue: 245 }, ps4: { name: 'PlayStation 4', short: 'PS4', hue: 235 },
  psp: { name: 'PSP', short: 'PSP', hue: 225 }, vita: { name: 'PS Vita', short: 'Vita', hue: 215 },
  xbox: { name: 'Xbox', short: 'Xbox', hue: 140 }, x360: { name: 'Xbox 360', short: 'X360', hue: 130 },
  xone: { name: 'Xbox One', short: 'XOne', hue: 145 }, dc: { name: 'Dreamcast', short: 'DC', hue: 40 },
  saturn: { name: 'Sega Saturn', short: 'Saturn', hue: 260 },
}
/** Accent color for a console (dots, placeholders). */
export const platColor = (p: string) => `oklch(0.68 0.16 ${PLATFORMS[p]?.hue ?? hueOf(p)})`

export const platName = (p: string) => PLATFORMS[p]?.name ?? p.toUpperCase()
export const platShort = (p: string) => PLATFORMS[p]?.short ?? p.toUpperCase()

/** Stable hue from a string, for placeholder covers. */
export function hueOf(s: string): number {
  let h = 0
  for (const c of s) h = (h * 31 + c.charCodeAt(0)) % 360
  return h
}

export const inScope = (scope: string, root: string) => scope === root || scope.startsWith(root + '/')

export const words = (s: string) => s.toLowerCase().split(/[^a-z0-9_]+/).filter(Boolean)
export function matches(q: string, ...fields: (string | null | undefined)[]) {
  const t = words(q)
  if (!t.length) return true
  const hay = fields.map((f) => (f ?? '').toLowerCase()).join(' \n ')
  return t.every((w) => hay.includes(w))
}

export const STATUS_ORDER = ['planned', 'in-progress', 'boots', 'playable', 'complete']
export const STATUS_LABEL: Record<string, string> = {
  planned: 'Planned', 'in-progress': 'In progress', boots: 'Boots', playable: 'Playable', complete: 'Complete',
}
