import { Wrench } from 'lucide-react'
import type { Data } from '@/lib/data'
import { withParam, type Route } from '@/lib/router'
import { AttemptCard } from '@/components/attempt'
import { Avatar, EmptyState, ExtLink, HardwareChips, PageHeader, ScopeLabel, TimeAgo } from '@/components/common'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'

export function Attempts({ data, route }: { data: Data; route: Route }) {
  const vendor = route.params.get('v') ?? ''
  const vendors = [...new Set(data.attempts.flatMap((a) => a.hw?.gpu_vendors ?? []))].sort()
  const all = data.attempts.filter((a) => !vendor || (a.hw?.gpu_vendors ?? []).includes(vendor))
  const by = (s: string) => all.filter((a) => a.state === s).sort((a, b) => b.updated_at.localeCompare(a.updated_at))
  const groups: [string, string][] = [['active', 'Active'], ['paused', 'Paused'], ['expired', 'Expired, closing soon unless renewed']]

  return (
    <>
      <PageHeader title="Attempts" description="Attempts are not locks: several people can try different approaches on the same problem." />
      {vendors.length > 0 && (
        <div className="mb-6 flex flex-wrap gap-1.5">
          <Button variant="chip" data-on={!vendor} onClick={() => withParam(route, 'v', null)}>Any GPU</Button>
          {vendors.map((v) => (
            <Button key={v} variant="chip" data-on={vendor === v} onClick={() => withParam(route, 'v', v)}>{v.toUpperCase()}</Button>
          ))}
        </div>
      )}
      <div className="space-y-8">
        {groups.map(([state, label]) => {
          const list = by(state)
          if (state !== 'active' && !list.length) return null
          return (
            <section key={state}>
              <h2 className="mb-3 text-sm font-semibold">{label} <span className="text-muted-foreground">({list.length})</span></h2>
              {list.length ? (
                <div className="grid gap-3 md:grid-cols-2">{list.map((a) => <AttemptCard key={a.repo + a.number} a={a} data={data} />)}</div>
              ) : (
                <EmptyState icon={Wrench} title="No active attempts" />
              )}
            </section>
          )
        })}
        <section>
          <h2 className="mb-3 text-sm font-semibold">Recently closed</h2>
          {data.closed_attempts.length ? (
            <Card className="divide-y">
              {data.closed_attempts.slice(0, 30).map((a) => (
                <div key={a.repo + a.number} className="flex flex-wrap items-center justify-between gap-3 p-4">
                  <div className="flex min-w-0 items-center gap-3">
                    <Avatar login={a.author} size={28} />
                    <div className="min-w-0">
                      <ExtLink href={a.url} className="font-medium">{a.title.replace(/^\[attempt\]\s*/, '')}</ExtLink>
                      <div className="flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
                        <ScopeLabel scope={a.scope} data={data} /> <span>&middot; closed <TimeAgo iso={a.closed_at} /></span>
                      </div>
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
            <EmptyState title="Nothing closed yet" />
          )}
        </section>
      </div>
    </>
  )
}
