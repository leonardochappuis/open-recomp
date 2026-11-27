// Ranks games by recent activity: last 7 days against the 7 days before,
// so the home page can show positions and movement like a chart.
import { inScope, type ActivityEvent, type Data, type Game } from '@/lib/data'

const WEIGHT: Record<string, number> = {
  finding: 3, reproduced: 2, 'not-reproduced': 2, disputed: 1, 'attempt-opened': 2,
  progress: 1, outcome: 2, 'attempt-closed': 1, comment: 0.5,
}
const DAY = 86400e3

export type Trend = { rank?: number; prevRank?: number; score: number; isNew: boolean; lastActive?: string }

function scopeOf(e: ActivityEvent, data: Data): string | undefined {
  if (e.target.finding) return data.findings.find((f) => f.id === e.target.finding && f.repo === e.repo)?.scope
  if (e.target.attempt) return [...data.attempts, ...data.closed_attempts].find((a) => a.number === e.target.attempt && a.repo === e.repo)?.scope
}

export function trending(data: Data): Map<string, Trend> {
  const now = Date.now()
  const cur = new Map<string, number>()
  const prev = new Map<string, number>()
  const last = new Map<string, string>()
  for (const e of data.activity) {
    const s = scopeOf(e, data)
    if (!s) continue
    const g = data.games.find((x) => inScope(s, x.scope))
    if (!g) continue
    const age = now - Date.parse(e.time)
    const w = WEIGHT[e.type] ?? 0.5
    if (age <= 7 * DAY) cur.set(g.scope, (cur.get(g.scope) ?? 0) + w)
    else if (age <= 14 * DAY) prev.set(g.scope, (prev.get(g.scope) ?? 0) + w)
    if (!last.has(g.scope) || e.time > last.get(g.scope)!) last.set(g.scope, e.time)
  }
  // Live work counts too, even without new events.
  for (const a of data.attempts.filter((a) => a.state === 'active')) {
    const g = data.games.find((x) => inScope(a.scope, x.scope))
    if (g) cur.set(g.scope, (cur.get(g.scope) ?? 0) + 2)
  }
  const ranked = (m: Map<string, number>) =>
    new Map([...m.entries()].filter(([, v]) => v > 0).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).map(([k], i) => [k, i + 1]))
  const r = ranked(cur)
  const p = ranked(prev)
  const out = new Map<string, Trend>()
  for (const g of data.games) {
    const added = g.added ? Date.parse(g.added) : NaN
    out.set(g.scope, {
      rank: r.get(g.scope),
      prevRank: p.get(g.scope),
      score: cur.get(g.scope) ?? 0,
      isNew: (!isNaN(added) && now - added < 14 * DAY) || (r.has(g.scope) && !p.has(g.scope)),
      lastActive: last.get(g.scope),
    })
  }
  return out
}

export const displayTitle = (g: Game) => g.title || g.name
