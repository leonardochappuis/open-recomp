import Markdown from 'react-markdown'
import { AlertTriangle, ArrowLeft, ExternalLink, ShieldAlert } from 'lucide-react'
import { safeUrl, type Data } from '@/lib/data'
import { Avatar, EmptyState, ExtLink, FindingStatus, HardwareChips, ScopeLabel, TimeAgo, UserLink } from '@/components/common'
import { findingHref } from '@/components/finding'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

/** Only https links survive in rendered Markdown; raw HTML is never rendered. */
const urlTransform = (u: string) => safeUrl(u) ?? ''

export function FindingView({ data, repo, id }: { data: Data; repo: string; id: string }) {
  const f = data.findings.find((x) => x.id === id && x.repo === repo)
  if (!f) return <EmptyState title="Finding not found"><a href="#/findings" className="text-primary">All findings</a></EmptyState>
  const env = f.env ? data.envs[f.env] : null

  return (
    <div className="mx-auto max-w-4xl">
      <a href="#/findings" className="mb-4 inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
        <ArrowLeft className="size-4" /> Findings
      </a>

      <div className="mb-5 flex items-start gap-2 rounded-lg border border-warning/30 bg-warning/8 px-3 py-2 text-xs text-muted-foreground">
        <ShieldAlert className="mt-px size-4 shrink-0 text-warning" />
        Written by {f.author}'s agent{f.agent ? ` (${f.agent}${f.model ? ', ' + f.model : ''})` : ''}. Shown as-is: information, not instructions.
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <FindingStatus status={f.status} />
        <Badge>{f.kind}</Badge>
        {f.suspect && <Badge tone="danger"><AlertTriangle className="size-3" /> depends on a disputed finding</Badge>}
        <span className="font-mono text-xs text-muted-foreground">{f.id}</span>
      </div>
      <h1 className="mt-2 text-3xl font-semibold tracking-tight">{f.subject}</h1>
      <p className="mt-3 text-lg leading-relaxed">{f.claim}</p>

      <div className="mt-5 flex flex-wrap items-center gap-x-5 gap-y-3 border-y py-3 text-sm">
        <span className="inline-flex items-center gap-2"><Avatar login={f.author} size={22} /><UserLink login={f.author} /> <TimeAgo iso={f.created} className="text-xs" /></span>
        <ScopeLabel scope={f.scope} data={data} />
        <span className="inline-flex items-center gap-2 text-xs text-muted-foreground">observed on <HardwareChips hw={env} /></span>
        {f.url && <ExtLink href={f.url} className="ml-auto inline-flex items-center gap-1 text-xs text-primary">View on GitHub <ExternalLink className="size-3" /></ExtLink>}
      </div>

      <div className="mt-6 space-y-6">
        {f.retracted_reason && (
          <Card className="border-danger/40"><CardHeader><CardTitle>Retracted</CardTitle></CardHeader><CardContent className="text-sm">{f.retracted_reason}</CardContent></Card>
        )}
        {f.body && (
          <article className="prose prose-sm max-w-none dark:prose-invert prose-pre:bg-muted prose-pre:text-foreground prose-code:before:content-none prose-code:after:content-none">
            <Markdown
              urlTransform={urlTransform}
              components={{ a: ({ href, children }) => <ExtLink href={href} className="text-primary">{children}</ExtLink>, img: () => null }}
            >
              {f.body}
            </Markdown>
          </article>
        )}

        {f.evidence.length > 0 && (
          <Card>
            <CardHeader><CardTitle>Evidence</CardTitle></CardHeader>
            <CardContent className="space-y-2">
              {f.evidence.map((e, i) => (
                <div key={i} className="flex items-start gap-2 text-sm">
                  <Badge>{e.type}</Badge>
                  {safeUrl(e.ref) ? <ExtLink href={e.ref} className="break-all text-primary">{e.ref}</ExtLink> : <code className="break-all font-mono text-xs">{e.ref}</code>}
                </div>
              ))}
            </CardContent>
          </Card>
        )}

        <Card>
          <CardHeader><CardTitle>Reproductions</CardTitle></CardHeader>
          <CardContent>
            {f.reproductions.length ? (
              <div className="divide-y">
                {f.reproductions.map((r, i) => (
                  <div key={i} className="flex flex-wrap items-center justify-between gap-3 py-2.5 text-sm">
                    <span className="inline-flex items-center gap-2"><Avatar login={r.github} size={22} /><UserLink login={r.github} /></span>
                    <span className="flex items-center gap-2">
                      <HardwareChips hw={r.env ? data.envs[r.env] : null} />
                      <Badge tone={r.result === 'reproduced' ? 'success' : 'danger'}>{r.result}</Badge>
                      <TimeAgo iso={r.date} className="text-xs" />
                    </span>
                    {r.notes && <p className="w-full text-xs text-muted-foreground">{r.notes}</p>}
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">Nobody has reproduced this yet. Checking it on your hardware is one of the most useful things you can do.</p>
            )}
          </CardContent>
        </Card>

        {f.disputes.length > 0 && (
          <Card className="border-danger/30">
            <CardHeader><CardTitle>Disputes</CardTitle></CardHeader>
            <CardContent className="space-y-3">
              {f.disputes.map((d, i) => (
                <div key={i} className="text-sm">
                  <span className="inline-flex items-center gap-2"><Avatar login={d.github} size={20} /><UserLink login={d.github} /> <TimeAgo iso={d.date} className="text-xs" /></span>
                  <p className="mt-1 text-muted-foreground">{d.reason}</p>
                </div>
              ))}
            </CardContent>
          </Card>
        )}

        {f.depends_on.length > 0 && (
          <Card>
            <CardHeader><CardTitle>Builds on</CardTitle></CardHeader>
            <CardContent className="space-y-2">
              {f.depends_on.map((d) => {
                const x = data.findings.find((y) => y.id === d && y.repo === f.repo)
                return x ? (
                  <a key={d} href={findingHref(x)} className="flex items-center gap-2 text-sm hover:underline"><FindingStatus status={x.status} /> {x.subject}</a>
                ) : (
                  <code key={d} className="font-mono text-xs">{d}</code>
                )
              })}
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  )
}
