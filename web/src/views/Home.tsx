import * as React from 'react'
import { ChevronLeft, ChevronRight, Flame, Gamepad2, Search, Zap } from 'lucide-react'
import { cn } from '@/lib/utils'
import { hubOf, matches, platColor, platName, STATUS_LABEL, STATUS_ORDER, type Data } from '@/lib/data'
import { trending, type Trend } from '@/lib/trending'
import { withParam, type Route } from '@/lib/router'
import { Avatar, EmptyState, LiveDot, ScopeLabel } from '@/components/common'
import { GameTile } from '@/components/game-tile'
import { AddGameDialog } from '@/components/dialogs'
import { Button } from '@/components/ui/button'
import { gameStats } from './Games'

const SORTS = [
  ['trending', 'Trending'],
  ['active', 'Recently active'],
  ['new', 'Newest'],
  ['az', 'A to Z'],
] as const

function TrendingRow({ games, trends }: { games: ReturnType<typeof gameStats>; trends: Map<string, Trend> }) {
  const ref = React.useRef<HTMLDivElement>(null)
  const scroll = (dir: number) => ref.current?.scrollBy({ left: dir * ref.current.clientWidth * 0.8, behavior: 'smooth' })
  return (
    <div className="group/row relative">
      <div ref={ref} className="-mx-4 flex snap-x gap-4 overflow-x-auto px-4 pb-2 pt-2 [scrollbar-width:none] sm:-mx-6 sm:px-6">
        {games.map((g) => (
          <GameTile key={g.scope} g={g} trend={trends.get(g.scope)} showRank className="w-[150px] shrink-0 snap-start sm:w-[170px]" />
        ))}
      </div>
      {games.length > 5 && (
        <>
          <Button variant="outline" size="icon" onClick={() => scroll(-1)} aria-label="Scroll left" className="absolute -left-3 top-[38%] hidden rounded-full opacity-0 shadow-lg transition-opacity group-hover/row:opacity-100 sm:inline-flex">
            <ChevronLeft className="size-4" />
          </Button>
          <Button variant="outline" size="icon" onClick={() => scroll(1)} aria-label="Scroll right" className="absolute -right-3 top-[38%] hidden rounded-full opacity-0 shadow-lg transition-opacity group-hover/row:opacity-100 sm:inline-flex">
            <ChevronRight className="size-4" />
          </Button>
        </>
      )}
    </div>
  )
}

function WorkingNow({ data }: { data: Data }) {
  const active = data.attempts.filter((a) => a.state === 'active').sort((a, b) => b.updated_at.localeCompare(a.updated_at))
  if (!active.length) return null
  return (
    <section className="mb-12">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="flex items-center gap-2 text-lg font-semibold tracking-tight">
          <LiveDot /> Working on right now
        </h2>
        <a href="#/live" className="text-sm text-muted-foreground hover:text-foreground">Live activity</a>
      </div>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {active.slice(0, 6).map((a) => (
          <a
            key={a.repo + a.number}
            href={a.url}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-3 rounded-xl border bg-card/70 p-3 transition-colors hover:border-glow/60"
          >
            <span className="relative">
              <Avatar login={a.author} size={36} />
              <span className="absolute -bottom-0.5 -right-0.5 grid size-4 place-items-center rounded-full bg-glow text-zinc-950 ring-2 ring-card">
                <Zap className="size-2.5 fill-current" />
              </span>
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-sm font-medium">{a.title.replace(/^\[attempt\]\s*/, '')}</span>
              <span className="block truncate text-xs text-muted-foreground">
                {a.author} &middot; <ScopeLabel scope={a.scope} data={data} />
              </span>
            </span>
          </a>
        ))}
      </div>
    </section>
  )
}

export function Home({ data, route }: { data: Data; route: Route }) {
  const q = route.params.get('q') ?? ''
  const plat = route.params.get('p') ?? ''
  const st = route.params.get('s') ?? ''
  const sort = route.params.get('sort') ?? 'trending'
  const games = gameStats(data)
  const trends = trending(data)
  const consoles = [...new Set(games.map((g) => g.platform))].sort((a, b) => platName(a).localeCompare(platName(b)))
  const people = new Set(data.attempts.filter((a) => a.state === 'active').map((a) => a.author)).size

  const chart = games
    .filter((g) => trends.get(g.scope)?.rank)
    .sort((a, b) => trends.get(a.scope)!.rank! - trends.get(b.scope)!.rank!)
    .slice(0, 12)

  const t = (s: string) => trends.get(s)
  const shown = games
    .filter((g) => (!plat || g.platform === plat) && (!st || g.status === st))
    .filter((g) => matches(q, g.name, g.title, g.id, g.platform, platName(g.platform), (g.tags ?? []).join(' '), (g.targets ?? []).join(' ')))
    .sort((a, b) => {
      if (sort === 'az') return (a.title || a.name).localeCompare(b.title || b.name)
      if (sort === 'new') return (b.added ?? '').localeCompare(a.added ?? '') || a.name.localeCompare(b.name)
      if (sort === 'active') return (t(b.scope)?.lastActive ?? '').localeCompare(t(a.scope)?.lastActive ?? '') || a.name.localeCompare(b.name)
      return (t(a.scope)?.rank ?? 1e9) - (t(b.scope)?.rank ?? 1e9) || b.live - a.live || a.name.localeCompare(b.name)
    })

  return (
    <>
      <section className="relative mb-12 pt-4 text-center sm:pt-10">
        <div className="mx-auto mb-4 inline-flex items-center gap-2 rounded-full border bg-card/70 px-3 py-1 text-xs text-muted-foreground">
          <LiveDot /> {people > 0 ? `${people} ${people === 1 ? 'person' : 'people'} working right now` : 'Live from every project'}
        </div>
        <h1 className="mx-auto max-w-3xl text-4xl font-semibold tracking-tight text-balance sm:text-5xl">
          Console games,{' '}
          <span className="bg-gradient-to-r from-primary via-info to-glow bg-clip-text text-transparent">recompiled together</span>
        </h1>
        <p className="mx-auto mt-4 max-w-2xl text-balance text-muted-foreground">
          Every recompilation project in one place: what's playable, who is working on what right now, and everything their agents have learned.
        </p>
        <div className="relative mx-auto mt-7 max-w-xl">
          <Search className="pointer-events-none absolute left-4 top-1/2 z-10 size-5 -translate-y-1/2 text-muted-foreground" />
          <input
            id="home-q"
            type="search"
            value={q}
            onChange={(e) => withParam(route, 'q', e.target.value)}
            placeholder="Search games, recomps, consoles..."
            aria-label="Search games"
            className="h-12 w-full rounded-2xl border bg-card/80 pl-12 pr-4 text-base shadow-lg shadow-black/5 backdrop-blur placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-glow"
          />
        </div>
        <div className="mt-5 flex flex-wrap items-center justify-center gap-3">
          <AddGameDialog hub={hubOf(data)} />
          <a href="#/live" className="inline-flex h-9 items-center gap-1.5 rounded-md border bg-card/70 px-3.5 text-sm font-medium hover:bg-accent">
            <LiveDot /> See what's happening
          </a>
        </div>
        <div className="mt-5 flex flex-wrap justify-center gap-x-6 gap-y-1 text-sm text-muted-foreground">
          <span><b className="font-semibold text-foreground tabular-nums">{games.length}</b> games</span>
          <span><b className="font-semibold text-foreground tabular-nums">{consoles.length}</b> consoles</span>
          <span><b className="font-semibold text-foreground tabular-nums">{data.findings.length}</b> findings</span>
          <span><b className="font-semibold text-foreground tabular-nums">{data.findings.filter((f) => f.status === 'verified').length}</b> verified</span>
        </div>
      </section>

      {!q && chart.length > 0 && (
        <section className="mb-12">
          <div className="mb-2 flex items-end justify-between">
            <div>
              <h2 className="flex items-center gap-2 text-lg font-semibold tracking-tight">
                <Flame className="size-5 text-amber-400" /> Trending this week
              </h2>
              <p className="text-sm text-muted-foreground">Ranked by findings, attempts and progress over the last 7 days.</p>
            </div>
          </div>
          <TrendingRow games={chart} trends={trends} />
        </section>
      )}

      {!q && <WorkingNow data={data} />}

      <section id="all-games">
        <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
          <div className="flex items-baseline gap-3">
            <h2 className="text-lg font-semibold tracking-tight">{q ? `Results for "${q}"` : 'All games'}</h2>
            {(data.requests ?? []).length > 0 && (
              <a href="#/unfiltered" className="text-xs text-warning hover:underline">
                {(data.requests ?? []).length} unreviewed submission{(data.requests ?? []).length === 1 ? '' : 's'}
              </a>
            )}
          </div>
          <div className="flex flex-wrap gap-1 rounded-lg border bg-card/60 p-1">
            {SORTS.map(([k, label]) => (
              <button
                key={k}
                onClick={() => withParam(route, 'sort', k === 'trending' ? null : k)}
                className={cn('rounded-md px-2.5 py-1 text-xs font-medium text-muted-foreground transition-colors hover:text-foreground cursor-pointer', sort === k && 'bg-accent text-foreground')}
              >
                {label}
              </button>
            ))}
          </div>
        </div>
        {games.length > 0 && (
          <div className="mb-6 space-y-2">
            {consoles.length > 1 && (
              <div className="flex flex-wrap gap-1.5">
                <Button variant="chip" data-on={!plat} onClick={() => withParam(route, 'p', null)}>All consoles</Button>
                {consoles.map((p) => (
                  <Button key={p} variant="chip" data-on={plat === p} onClick={() => withParam(route, 'p', plat === p ? null : p)}>
                    <span className="mr-1.5 size-2 rounded-full" style={{ background: platColor(p) }} />
                    {platName(p)}
                  </Button>
                ))}
              </div>
            )}
            <div className="flex flex-wrap gap-1.5">
              <Button variant="chip" data-on={!st} onClick={() => withParam(route, 's', null)}>Any status</Button>
              {STATUS_ORDER.map((s) => (
                <Button key={s} variant="chip" data-on={st === s} onClick={() => withParam(route, 's', st === s ? null : s)}>{STATUS_LABEL[s]}</Button>
              ))}
            </div>
          </div>
        )}
        {!games.length ? (
          <EmptyState icon={Gamepad2} title="No games in the catalog yet">
            Add the first one; a maintainer reviews it:
            <code className="mt-3 block rounded-md bg-muted px-3 py-2 text-left font-mono text-xs text-foreground">
              orc game add --platform x360 --id 4D5307E6 --name "Game name" --title "Recomp name" --target windows --target linux --cover-file cover.png
            </code>
          </EmptyState>
        ) : !shown.length ? (
          <EmptyState icon={Search} title="No games match">Try another search or filter.</EmptyState>
        ) : (
          <div className="grid grid-cols-2 gap-x-4 gap-y-6 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
            {shown.map((g) => <GameTile key={g.repo + g.scope} g={g} trend={trends.get(g.scope)} />)}
          </div>
        )}
      </section>
    </>
  )
}
