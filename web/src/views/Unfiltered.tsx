import * as React from 'react'
import { AlertTriangle, Eye, EyeOff, MessageSquare } from 'lucide-react'
import { hubOf, type Data } from '@/lib/data'
import { Avatar, Cover, EmptyState, ExtLink, GameStatus, PageHeader, TimeAgo } from '@/components/common'
import { AddGameDialog } from '@/components/dialogs'
import { ConsoleLabel, Targets } from '@/components/game-tile'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'

export function Unfiltered({ data }: { data: Data }) {
  const [covers, setCovers] = React.useState(false)
  const reqs = [...(data.requests ?? [])].sort((a, b) => b.created_at.localeCompare(a.created_at))
  const valid = reqs.filter((r) => r.game)
  const broken = reqs.filter((r) => !r.game)
  const hub = hubOf(data)

  return (
    <>
      <PageHeader title="Unfiltered" description="Games people submitted that a maintainer hasn't reviewed yet.">
        <div className="flex gap-2">
          <Button variant="outline" onClick={() => setCovers((c) => !c)}>
            {covers ? <EyeOff className="size-4" /> : <Eye className="size-4" />} {covers ? 'Hide covers' : 'Show covers'}
          </Button>
          <AddGameDialog hub={hub} />
        </div>
      </PageHeader>

      <div className="mb-8 flex gap-3 rounded-xl border border-warning/40 bg-warning/10 p-4 text-sm">
        <AlertTriangle className="mt-0.5 size-5 shrink-0 text-warning" />
        <div>
          <div className="font-semibold">Not reviewed</div>
          <p className="text-muted-foreground">
            Anyone can submit these. Names, covers and links haven't been checked and may be wrong, duplicated or inappropriate. Covers are hidden until you choose to show them. Don't download anything linked from here unless you trust it.
          </p>
        </div>
      </div>

      {!reqs.length ? (
        <EmptyState title="Nothing waiting for review">New submissions from the Add a game form show up here.</EmptyState>
      ) : (
        <>
          {valid.length > 0 && (
            <div className="grid grid-cols-2 gap-x-4 gap-y-6 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
              {valid.map((r) => {
                const g = r.game!
                return (
                  <a key={r.repo + r.number} href={r.url} target="_blank" rel="noopener noreferrer" className="group block">
                    <div className="relative overflow-hidden rounded-2xl ring-1 ring-warning/40 transition-all group-hover:ring-2 group-hover:ring-warning">
                      <Cover game={covers ? g : { ...g, cover: null }} bare />
                      <div className="pointer-events-none absolute inset-0 bg-gradient-to-t from-black/75 via-black/0 via-45% to-black/30" />
                      <div className="absolute inset-x-2 top-2 flex items-start justify-between gap-1">
                        <span className="rounded-md bg-warning px-1.5 py-1 text-[10px] font-bold uppercase tracking-wide text-zinc-950">Unreviewed</span>
                        <Targets targets={g.targets} />
                      </div>
                      <div className="absolute inset-x-2 bottom-2 flex items-center gap-1.5">
                        <span className="inline-flex items-center gap-1 rounded-full bg-black/60 px-2 py-1 text-[11px] text-white backdrop-blur">
                          <MessageSquare className="size-3" /> {r.comments}
                        </span>
                        <GameStatus status={g.status} />
                      </div>
                    </div>
                    <div className="mt-2.5 space-y-0.5 text-center">
                      <div className="truncate text-sm font-semibold">{g.title || g.name}</div>
                      {g.title && <div className="truncate text-xs text-muted-foreground">{g.name}</div>}
                      <ConsoleLabel platform={g.platform} className="justify-center text-xs text-muted-foreground" />
                      <div className="flex items-center justify-center gap-1.5 pt-1 text-[11px] text-muted-foreground">
                        <Avatar login={r.author} size={14} /> {r.author} &middot; <TimeAgo iso={r.created_at} />
                      </div>
                    </div>
                  </a>
                )
              })}
            </div>
          )}

          {broken.length > 0 && (
            <section className="mt-10">
              <h2 className="mb-3 text-sm font-semibold">Incomplete submissions</h2>
              <Card className="divide-y">
                {broken.map((r) => (
                  <div key={r.repo + r.number} className="p-4">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <ExtLink href={r.url} className="font-medium">{r.name || r.title}</ExtLink>
                      <span className="flex items-center gap-2 text-xs text-muted-foreground">
                        <Avatar login={r.author} size={16} /> {r.author} &middot; <TimeAgo iso={r.created_at} />
                      </span>
                    </div>
                    <div className="mt-2 flex flex-wrap gap-1.5">
                      {r.errors.slice(0, 4).map((e, i) => <Badge key={i} tone="danger">{e.replace(/^kb\/games\/[^:]+: /, '')}</Badge>)}
                    </div>
                  </div>
                ))}
              </Card>
            </section>
          )}
        </>
      )}
    </>
  )
}
