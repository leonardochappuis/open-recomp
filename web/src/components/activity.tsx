import { CheckCircle2, CircleDot, FlagTriangleRight, Lightbulb, MessageSquare, PlayCircle, ShieldAlert, XCircle, Trophy } from 'lucide-react'
import { cn } from '@/lib/utils'
import { eventKey, type ActivityEvent, type Data } from '@/lib/data'
import { Avatar, EmptyState, ExtLink, ScopeLabel, TimeAgo, UserLink } from '@/components/common'
import { Badge } from '@/components/ui/badge'

const KIND: Record<string, { icon: React.ElementType; color: string; verb: string }> = {
  finding: { icon: Lightbulb, color: 'text-primary bg-primary/12', verb: 'shared a finding' },
  reproduced: { icon: CheckCircle2, color: 'text-success bg-success/12', verb: 'reproduced' },
  'not-reproduced': { icon: XCircle, color: 'text-danger bg-danger/12', verb: 'could not reproduce' },
  disputed: { icon: ShieldAlert, color: 'text-danger bg-danger/12', verb: 'disputed' },
  'attempt-opened': { icon: PlayCircle, color: 'text-warning bg-warning/12', verb: 'started working on' },
  'attempt-closed': { icon: FlagTriangleRight, color: 'text-muted-foreground bg-muted', verb: 'closed' },
  progress: { icon: CircleDot, color: 'text-info bg-info/12', verb: 'posted progress on' },
  outcome: { icon: Trophy, color: 'text-success bg-success/12', verb: 'reported the outcome of' },
  comment: { icon: MessageSquare, color: 'text-muted-foreground bg-muted', verb: 'commented on' },
}

function scopeOf(e: ActivityEvent, data: Data) {
  if (e.target.finding) return data.findings.find((f) => f.id === e.target.finding && f.repo === e.repo)?.scope
  if (e.target.attempt) {
    return [...data.attempts, ...data.closed_attempts].find((a) => a.number === e.target.attempt && a.repo === e.repo)?.scope
  }
}

export function eventScope(e: ActivityEvent, data: Data) {
  return scopeOf(e, data)
}

function Target({ e, data }: { e: ActivityEvent; data: Data }) {
  if (e.target.finding) {
    const f = data.findings.find((x) => x.id === e.target.finding && x.repo === e.repo)
    return f ? (
      <a className="font-medium text-foreground hover:underline" href={`#/finding/${encodeURIComponent(f.repo)}/${encodeURIComponent(f.id)}`}>
        {e.text}
      </a>
    ) : (
      <span className="font-medium">{e.text}</span>
    )
  }
  if (e.target.attempt) {
    const a = [...data.attempts, ...data.closed_attempts].find((x) => x.number === e.target.attempt && x.repo === e.repo)
    const title = (a?.title ?? `#${e.target.attempt}`).replace(/^\[attempt\]\s*/, '')
    return (
      <ExtLink href={e.target.url ?? a?.url} className="font-medium text-foreground">
        {title}
      </ExtLink>
    )
  }
  return <span className="font-medium">{e.text}</span>
}

export function ActivityFeed({ events, data, fresh, limit = 60 }: { events: ActivityEvent[]; data: Data; fresh: Set<string>; limit?: number }) {
  if (!events.length) return <EmptyState title="No activity yet">Findings, attempts and progress will show up here as they happen.</EmptyState>
  return (
    <ol className="relative">
      {events.slice(0, limit).map((e, i) => {
        const k = KIND[e.type] ?? KIND.comment
        const Icon = k.icon
        const scope = scopeOf(e, data)
        const showText = ['progress', 'comment', 'outcome'].includes(e.type) && e.text
        const reason = e.type === 'attempt-closed' ? e.target.reason : null
        return (
          <li key={eventKey(e) + i} className={cn('group relative flex gap-3 rounded-md py-3 pl-1 pr-2', fresh.has(eventKey(e)) && 'flash')}>
            {i < Math.min(events.length, limit) - 1 && <span className="absolute left-[19px] top-10 bottom-0 w-px bg-border" aria-hidden />}
            <span className={cn('relative z-10 grid size-8 shrink-0 place-items-center rounded-full ring-4 ring-card', k.color)}>
              <Icon className="size-4" />
            </span>
            <div className="min-w-0 flex-1">
              <div className="flex items-start justify-between gap-3">
                <p className="min-w-0 text-sm leading-6">
                  <span className="mr-1.5 inline-flex translate-y-[3px]"><Avatar login={e.who} size={18} /></span>
                  <UserLink login={e.who} /> <span className="text-muted-foreground">{k.verb}</span> <Target e={e} data={data} />
                  {reason && (
                    <Badge tone={reason === 'completed' ? 'success' : 'neutral'} className="ml-1.5 align-[1px]">
                      {reason === 'completed' ? 'completed' : 'not planned'}
                    </Badge>
                  )}
                </p>
                <TimeAgo iso={e.time} className="shrink-0 pt-0.5 text-xs" />
              </div>
              {showText && <p className="mt-1 line-clamp-3 rounded-md bg-muted/60 px-3 py-2 text-[13px] text-muted-foreground">{e.text}</p>}
              <div className="mt-1 text-xs">{scope ? <ScopeLabel scope={scope} data={data} /> : <span className="text-muted-foreground">{e.repo}</span>}</div>
            </div>
          </li>
        )
      })}
    </ol>
  )
}
