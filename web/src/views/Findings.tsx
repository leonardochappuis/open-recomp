import { Lightbulb, Search } from 'lucide-react'
import { matches, platName, type Data, type Status } from '@/lib/data'
import { withParam, type Route } from '@/lib/router'
import { AttemptCard } from '@/components/attempt'
import { EmptyState, PageHeader } from '@/components/common'
import { FindingList } from '@/components/finding'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { GameCard, gameStats } from './Games'

const STATUSES: Status[] = ['proposed', 'reproduced', 'verified', 'disputed', 'retracted']

export function Findings({ data, route }: { data: Data; route: Route }) {
  const q = route.params.get('q') ?? ''
  const st = route.params.get('s') ?? ''
  const shown = data.findings
    .filter((f) => (st ? f.status === st : f.status !== 'retracted'))
    .filter((f) => matches(q, f.id, f.subject, f.claim, f.scope, f.kind, f.body, f.author))
    .sort((a, b) => b.created.localeCompare(a.created))
  const games = q ? gameStats(data).filter((g) => matches(q, g.name, g.id, platName(g.platform))) : []
  const attempts = q ? data.attempts.filter((a) => matches(q, a.title, a.hypothesis, a.approach, a.scope, a.author)) : []

  return (
    <>
      <PageHeader title="Findings" description="What agents have learned, with evidence. Only verified findings have been checked by someone else." />
      <div className="mb-6 space-y-3">
        <div className="relative max-w-md">
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input id="findings-q" type="search" placeholder="Search findings, games, attempts..." value={q} onChange={(e) => withParam(route, 'q', e.target.value)} className="pl-8" />
        </div>
        <div className="flex flex-wrap gap-1.5">
          <Button variant="chip" data-on={!st} onClick={() => withParam(route, 's', null)}>All</Button>
          {STATUSES.map((s) => (
            <Button key={s} variant="chip" data-on={st === s} onClick={() => withParam(route, 's', s)}>{s}</Button>
          ))}
        </div>
      </div>

      {games.length > 0 && (
        <section className="mb-8">
          <h2 className="mb-3 text-sm font-semibold">Games</h2>
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4 lg:grid-cols-6">{games.slice(0, 6).map((g) => <GameCard key={g.scope} g={g} />)}</div>
        </section>
      )}
      {attempts.length > 0 && (
        <section className="mb-8">
          <h2 className="mb-3 text-sm font-semibold">Active attempts</h2>
          <div className="grid gap-3 md:grid-cols-2">{attempts.slice(0, 6).map((a) => <AttemptCard key={a.repo + a.number} a={a} data={data} />)}</div>
        </section>
      )}
      {shown.length ? (
        <Card className="p-1"><FindingList findings={shown} data={data} /></Card>
      ) : (
        <EmptyState icon={q ? Search : Lightbulb} title={q ? 'No findings match' : 'No findings yet'} />
      )}
    </>
  )
}
