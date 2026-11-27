import { ArrowLeft, Cpu } from 'lucide-react'
import { hubOf, inScope, safeUrl, type Data, type Hw } from '@/lib/data'
import { withParam, type Route } from '@/lib/router'
import { ActivityFeed, eventScope } from '@/components/activity'
import { AttemptCard } from '@/components/attempt'
import { Avatar, Cover, EmptyState, ExtLink, GameStatus, HardwareChips, TimeAgo, UserLink } from '@/components/common'
import { FindingList } from '@/components/finding'
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { gameStats } from './Games'
import { ConsoleLabel, Targets } from '@/components/game-tile'
import { ContributeDialog } from '@/components/dialogs'

type Row = { reported: number; reproduced: number; not: number; active: number }

function HardwareMatrix({ data, scope }: { data: Data; scope: string }) {
  const rows = new Map<string, Row>()
  const bump = (hw: Hw | null | undefined, key: keyof Row) => {
    const vendors = hw?.gpu_vendors?.length ? hw.gpu_vendors : ['unknown']
    for (const v of vendors) {
      const k = `${v}|${hw?.os ?? 'unknown'}`
      const r = rows.get(k) ?? { reported: 0, reproduced: 0, not: 0, active: 0 }
      r[key]++
      rows.set(k, r)
    }
  }
  for (const f of data.findings.filter((f) => inScope(f.scope, scope))) {
    bump(f.env ? data.envs[f.env] : null, 'reported')
    for (const r of f.reproductions) bump(r.env ? data.envs[r.env] : null, r.result === 'reproduced' ? 'reproduced' : 'not')
  }
  for (const a of data.attempts.filter((a) => inScope(a.scope, scope))) bump(a.hw, 'active')
  const keys = [...rows.keys()].sort()
  if (!keys.length) return <EmptyState icon={Cpu} title="No hardware data yet">It fills in as people report findings and reproductions on their machines.</EmptyState>
  const max = Math.max(1, ...[...rows.values()].flatMap((r) => [r.reported, r.reproduced, r.not, r.active]))
  const cell = (n: number, tone: string) => (
    <td className="px-3 py-2.5">
      <div className="flex items-center gap-2">
        <div className="h-1.5 w-16 overflow-hidden rounded-full bg-muted">
          <div className={`h-full rounded-full ${tone}`} style={{ width: `${(n / max) * 100}%` }} />
        </div>
        <span className="tabular-nums">{n}</span>
      </div>
    </td>
  )
  return (
    <Card className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b text-left text-xs text-muted-foreground">
            <th className="px-3 py-2.5 font-medium">Hardware</th>
            <th className="px-3 py-2.5 font-medium">Findings seen on</th>
            <th className="px-3 py-2.5 font-medium">Reproduced</th>
            <th className="px-3 py-2.5 font-medium">Not reproduced</th>
            <th className="px-3 py-2.5 font-medium">Working now</th>
          </tr>
        </thead>
        <tbody>
          {keys.map((k) => {
            const [v, os] = k.split('|')
            const r = rows.get(k)!
            return (
              <tr key={k} className="border-b last:border-0">
                <td className="px-3 py-2.5"><HardwareChips hw={{ gpu_vendors: v === 'unknown' ? [] : [v], os, summary: `${v} on ${os}` }} /></td>
                {cell(r.reported, 'bg-primary')}
                {cell(r.reproduced, 'bg-success')}
                {cell(r.not, 'bg-danger')}
                {cell(r.active, 'bg-warning')}
              </tr>
            )
          })}
        </tbody>
      </table>
      <p className="border-t px-3 py-2.5 text-xs text-muted-foreground">
        A finding reproduced on one GPU vendor but not another usually means the fix is hardware-specific.
      </p>
    </Card>
  )
}

export function GameView({ data, route, scope, fresh }: { data: Data; route: Route; scope: string; fresh: Set<string> }) {
  const g = gameStats(data).find((x) => x.scope === scope)
  if (!g) return <EmptyState title="Game not found"><a href="#/games" className="text-primary">All games</a></EmptyState>
  const tab = route.params.get('tab') ?? 'active'
  const findings = data.findings.filter((f) => inScope(f.scope, g.scope)).sort((a, b) => b.created.localeCompare(a.created))
  const active = data.attempts.filter((a) => inScope(a.scope, g.scope))
  const closed = data.closed_attempts.filter((a) => inScope(a.scope, g.scope))
  const events = data.activity.filter((e) => { const s = eventScope(e, data); return !!s && inScope(s, g.scope) })
  const cover = safeUrl(g.cover)

  return (
    <>
      <a href="#/" className="mb-4 inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
        <ArrowLeft className="size-4" /> Games
      </a>
      <section className="relative mb-8 overflow-hidden rounded-xl border bg-card">
        {cover && <img src={cover} alt="" aria-hidden referrerPolicy="no-referrer" className="absolute inset-0 size-full scale-110 object-cover opacity-25 blur-2xl" />}
        <div className="relative flex flex-col gap-6 p-5 sm:flex-row sm:p-6">
          <Cover game={g} large className="w-36 shrink-0 rounded-lg border shadow-xl sm:w-44" />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
              <ConsoleLabel platform={g.platform} />
              <span>&middot;</span>
              <span className="font-mono text-xs">{g.id}</span>
              {g.region && <><span>&middot;</span><span>{g.region}</span></>}
              {g.year && <><span>&middot;</span><span>{g.year}</span></>}
            </div>
            <h1 className="mt-1 text-3xl font-semibold tracking-tight sm:text-4xl">{g.title || g.name}</h1>
            {g.title && <p className="text-lg text-muted-foreground">{g.name}</p>}
            <div className="mt-3 flex flex-wrap items-center gap-2">
              <GameStatus status={g.status} />
              <Targets targets={g.targets} className="bg-muted text-foreground backdrop-blur-none" />
              {(g.tags ?? []).map((t) => <Badge key={t}>{t}</Badge>)}
            </div>
            {g.description && <p className="mt-3 max-w-2xl text-sm text-muted-foreground">{g.description}</p>}
            <div className="mt-5 grid max-w-lg grid-cols-3 gap-3">
              {[['Working now', g.live], ['Findings', g.findings], ['Verified', g.verified]].map(([l, n]) => (
                <div key={l} className="rounded-lg border bg-background/50 px-3 py-2">
                  <div className="text-xl font-semibold tabular-nums">{n}</div>
                  <div className="text-xs text-muted-foreground">{l}</div>
                </div>
              ))}
            </div>
            <div className="mt-5 flex flex-wrap items-center gap-3">
              <ContributeDialog hub={hubOf(data)} game={g} />
              {g.live > 0 && <span className="text-sm text-muted-foreground">{g.live} {g.live === 1 ? 'person is' : 'people are'} working on it right now</span>}
            </div>
            {(g.repos ?? []).length > 0 && (
              <div className="mt-4 flex flex-wrap gap-x-4 gap-y-1 text-sm">
                {(g.repos ?? []).map((r) => (
                  <ExtLink key={r} href={`https://github.com/${r}`} className="font-mono text-xs text-primary">{r}</ExtLink>
                ))}
              </div>
            )}
          </div>
        </div>
      </section>

      <Tabs value={tab} onValueChange={(v) => withParam(route, 'tab', v === 'active' ? null : v)}>
        <TabsList>
          <TabsTrigger value="active">Working now <Badge>{active.length}</Badge></TabsTrigger>
          <TabsTrigger value="findings">Findings <Badge>{findings.length}</Badge></TabsTrigger>
          <TabsTrigger value="hardware">Hardware</TabsTrigger>
          <TabsTrigger value="activity">Activity</TabsTrigger>
          <TabsTrigger value="closed">Closed <Badge>{closed.length}</Badge></TabsTrigger>
        </TabsList>
        <TabsContent value="active">
          {active.length ? (
            <div className="grid gap-3 md:grid-cols-2">{active.map((a) => <AttemptCard key={a.repo + a.number} a={a} data={data} />)}</div>
          ) : (
            <EmptyState title="Nobody is working on this game right now" />
          )}
        </TabsContent>
        <TabsContent value="findings">
          {findings.length ? <Card className="p-1"><FindingList findings={findings} data={data} /></Card> : <EmptyState title="No findings yet" />}
        </TabsContent>
        <TabsContent value="hardware"><HardwareMatrix data={data} scope={g.scope} /></TabsContent>
        <TabsContent value="activity">
          <Card className="px-3 py-2"><ActivityFeed events={events} data={data} fresh={fresh} limit={150} /></Card>
        </TabsContent>
        <TabsContent value="closed">
          {closed.length ? (
            <Card className="divide-y">
              {closed.map((a) => (
                <div key={a.repo + a.number} className="flex flex-wrap items-center justify-between gap-3 p-4">
                  <div className="flex min-w-0 items-center gap-3">
                    <Avatar login={a.author} size={28} />
                    <div className="min-w-0">
                      <ExtLink href={a.url} className="font-medium">{a.title.replace(/^\[attempt\]\s*/, '')}</ExtLink>
                      <div className="text-xs text-muted-foreground"><UserLink login={a.author} /> &middot; closed <TimeAgo iso={a.closed_at} /></div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <HardwareChips hw={a.hw} compact />
                    <Badge tone={a.reason === 'completed' ? 'success' : 'neutral'}>{a.reason === 'completed' ? 'completed' : 'not planned'}</Badge>
                  </div>
                </div>
              ))}
            </Card>
          ) : (
            <EmptyState title="No closed attempts yet" />
          )}
        </TabsContent>
      </Tabs>
    </>
  )
}
