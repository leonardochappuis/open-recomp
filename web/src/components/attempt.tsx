import { ArrowUpRight, Clock, MessageSquare } from 'lucide-react'
import { cn } from '@/lib/utils'
import { ago, type Attempt, type Data } from '@/lib/data'
import { Avatar, ExtLink, HardwareChips, ScopeLabel, TimeAgo, UserLink } from '@/components/common'
import { Badge } from '@/components/ui/badge'
import { Card } from '@/components/ui/card'
import { Tooltip } from '@/components/ui/tooltip'

export function AttemptCard({ a, data, className }: { a: Attempt; data: Data; className?: string }) {
  const exp = Date.parse(a.lease_expires)
  const hours = a.lease_hours || 72
  const frac = Math.max(0, Math.min(1, (exp - Date.now()) / (hours * 3600e3)))
  const title = a.title.replace(/^\[attempt\]\s*/, '')
  return (
    <Card className={cn('group relative overflow-hidden p-4 transition-colors hover:border-primary/40', className)}>
      <div className="flex items-start gap-3">
        <Avatar login={a.author} size={32} />
        <div className="min-w-0 flex-1">
          <div className="flex items-start justify-between gap-2">
            <ExtLink href={a.url} className="font-semibold leading-snug text-foreground">
              {title}
            </ExtLink>
            <ArrowUpRight className="size-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
            <UserLink login={a.author} className="text-foreground/90" />
            <span>&middot;</span>
            <ScopeLabel scope={a.scope} data={data} />
          </div>
        </div>
      </div>

      {(a.hypothesis || a.approach) && (
        <dl className="mt-3 space-y-1.5 text-[13px]">
          {a.hypothesis && (
            <div className="flex gap-2">
              <dt className="w-20 shrink-0 text-[11px] font-medium uppercase tracking-wide text-muted-foreground pt-px">Hypothesis</dt>
              <dd className="min-w-0">{a.hypothesis}</dd>
            </div>
          )}
          {a.approach && (
            <div className="flex gap-2">
              <dt className="w-20 shrink-0 text-[11px] font-medium uppercase tracking-wide text-muted-foreground pt-px">Approach</dt>
              <dd className="min-w-0 text-muted-foreground">{a.approach}</dd>
            </div>
          )}
        </dl>
      )}

      <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
        <HardwareChips hw={a.hw} />
        <div className="flex items-center gap-3 text-xs text-muted-foreground">
          {a.state !== 'active' && <Badge tone={a.state === 'paused' ? 'warning' : 'neutral'}>{a.state}</Badge>}
          {a.comments > 0 && (
            <span className="inline-flex items-center gap-1">
              <MessageSquare className="size-3.5" /> {a.comments}
            </span>
          )}
          <span className="inline-flex items-center gap-1">
            <Clock className="size-3.5" /> <TimeAgo iso={a.updated_at} />
          </span>
        </div>
      </div>

      {a.state === 'active' && (
        <Tooltip content={`Lease ends ${ago(a.lease_expires)} (renews while the agent works)`}>
          <div className="absolute inset-x-0 bottom-0 h-0.5 bg-muted">
            <div className={cn('h-full transition-all', frac < 0.25 ? 'bg-warning' : 'bg-success')} style={{ width: `${frac * 100}%` }} />
          </div>
        </Tooltip>
      )}
    </Card>
  )
}
