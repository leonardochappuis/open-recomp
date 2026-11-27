import { AlertTriangle, CheckCheck } from 'lucide-react'
import type { Data, Finding } from '@/lib/data'
import { Avatar, FindingStatus, HardwareChips, ScopeLabel, TimeAgo } from '@/components/common'
import { Badge } from '@/components/ui/badge'

export const findingHref = (f: Finding) => `#/finding/${encodeURIComponent(f.repo)}/${encodeURIComponent(f.id)}`

export function FindingRow({ f, data }: { f: Finding; data: Data }) {
  const reps = f.reproductions.filter((r) => r.result === 'reproduced').length
  return (
    <li className="group">
      <a href={findingHref(f)} className="block rounded-md px-3 py-3.5 transition-colors hover:bg-accent/60">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-semibold">{f.subject}</span>
          <FindingStatus status={f.status} />
          <Badge>{f.kind}</Badge>
          {f.suspect && (
            <Badge tone="danger">
              <AlertTriangle className="size-3" /> suspect
            </Badge>
          )}
        </div>
        <p className="mt-1 text-sm text-muted-foreground line-clamp-2">{f.claim}</p>
        <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1.5">
            <Avatar login={f.author} size={16} /> {f.author}
          </span>
          <TimeAgo iso={f.created} />
          <span onClick={(e) => e.stopPropagation()} className="min-w-0">
            <ScopeLabel scope={f.scope} data={data} />
          </span>
          <HardwareChips hw={f.env ? data.envs[f.env] : null} compact />
          {reps > 0 && (
            <span className="inline-flex items-center gap-1 text-success">
              <CheckCheck className="size-3.5" /> reproduced by {reps}
            </span>
          )}
        </div>
      </a>
    </li>
  )
}

export function FindingList({ findings, data }: { findings: Finding[]; data: Data }) {
  return <ul className="divide-y">{findings.map((f) => <FindingRow key={f.repo + f.id} f={f} data={data} />)}</ul>
}
