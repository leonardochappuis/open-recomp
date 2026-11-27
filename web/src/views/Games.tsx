import { Gamepad2, Search } from 'lucide-react'
import { inScope, matches, platName, STATUS_LABEL, STATUS_ORDER, type Data, type Game } from '@/lib/data'
import { withParam, type Route } from '@/lib/router'
import { Cover, EmptyState, GameStatus, PageHeader } from '@/components/common'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

export type GameStats = Game & { live: number; findings: number; verified: number }

export function gameStats(data: Data): GameStats[] {
  const active = data.attempts.filter((a) => a.state === 'active')
  return data.games.map((g) => ({
    ...g,
    live: active.filter((a) => inScope(a.scope, g.scope)).length,
    findings: data.findings.filter((f) => inScope(f.scope, g.scope)).length,
    verified: data.findings.filter((f) => inScope(f.scope, g.scope) && f.status === 'verified').length,
  }))
}

export function GameCard({ g }: { g: GameStats }) {
  return (
    <a
      href={`#/game/${encodeURIComponent(g.scope)}`}
      className="group overflow-hidden rounded-lg border bg-card shadow-sm transition-all hover:-translate-y-0.5 hover:border-primary/50 hover:shadow-lg hover:shadow-primary/5"
    >
      <Cover game={g} live={g.live} className="transition-transform duration-300 group-hover:scale-[1.02]" />
      <div className="space-y-1.5 p-3">
        <div className="line-clamp-2 font-medium leading-snug">{g.name}</div>
        <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
          <GameStatus status={g.status} />
          <span>{g.findings} findings</span>
        </div>
      </div>
    </a>
  )
}

export function Games({ data, route }: { data: Data; route: Route }) {
  const q = route.params.get('q') ?? ''
  const plat = route.params.get('p') ?? ''
  const st = route.params.get('s') ?? ''
  const all = gameStats(data)
  const plats = [...new Set(all.map((g) => g.platform))].sort((a, b) => platName(a).localeCompare(platName(b)))
  const shown = all
    .filter((g) => (!plat || g.platform === plat) && (!st || g.status === st))
    .filter((g) => matches(q, g.name, g.id, g.platform, platName(g.platform), (g.tags ?? []).join(' ')))
    .sort((a, b) => b.live - a.live || a.name.localeCompare(b.name))

  return (
    <>
      <PageHeader title="Games" description="Every game someone is recompiling, on any console." />
      {all.length > 0 && (
        <div className="mb-6 space-y-3">
          <div className="relative max-w-md">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              id="games-q"
              type="search"
              placeholder="Search by name, title id, console..."
              value={q}
              onChange={(e) => withParam(route, 'q', e.target.value)}
              className="pl-8"
            />
          </div>
          {plats.length > 1 && (
            <div className="flex flex-wrap gap-1.5">
              <Button variant="chip" data-on={!plat} onClick={() => withParam(route, 'p', null)}>All consoles</Button>
              {plats.map((p) => (
                <Button key={p} variant="chip" data-on={plat === p} onClick={() => withParam(route, 'p', p)}>
                  {platName(p)}
                </Button>
              ))}
            </div>
          )}
          <div className="flex flex-wrap gap-1.5">
            <Button variant="chip" data-on={!st} onClick={() => withParam(route, 's', null)}>Any status</Button>
            {STATUS_ORDER.map((s) => (
              <Button key={s} variant="chip" data-on={st === s} onClick={() => withParam(route, 's', s)}>
                {STATUS_LABEL[s]}
              </Button>
            ))}
          </div>
        </div>
      )}
      {!all.length ? (
        <EmptyState icon={Gamepad2} title="No games in the catalog yet">
          Add one and a maintainer reviews it:
          <code className="mt-3 block rounded-md bg-muted px-3 py-2 text-left font-mono text-xs text-foreground">
            orc game add --platform x360 --id 4D5307E6 --name "Game name" --status in-progress --cover-file cover.png
          </code>
        </EmptyState>
      ) : !shown.length ? (
        <EmptyState icon={Search} title="No games match">Try another search or filter.</EmptyState>
      ) : (
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
          {shown.map((g) => <GameCard key={g.repo + g.scope} g={g} />)}
        </div>
      )}
    </>
  )
}
