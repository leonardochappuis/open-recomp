import { Activity, CheckCheck, Gamepad2, Lightbulb, Users, Wrench } from 'lucide-react'
import type { Data } from '@/lib/data'
import { ActivityFeed } from '@/components/activity'
import { AttemptCard } from '@/components/attempt'
import { EmptyState, ExtLink, LiveDot, TimeAgo } from '@/components/common'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

function Stat({ icon: Icon, value, label, accent }: { icon: React.ElementType; value: number; label: string; accent?: string }) {
  return (
    <Card className="relative overflow-hidden p-4">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground">{label}</span>
        <Icon className={`size-4 ${accent ?? 'text-muted-foreground'}`} />
      </div>
      <div className="mt-2 text-3xl font-semibold tabular-nums tracking-tight">{value}</div>
    </Card>
  )
}

export function Live({ data, fresh }: { data: Data; fresh: Set<string> }) {
  const active = data.attempts.filter((a) => a.state === 'active').sort((a, b) => b.updated_at.localeCompare(a.updated_at))
  const people = new Set(active.map((a) => a.author)).size
  const week = Date.now() - 7 * 86400e3
  const newFindings = data.findings.filter((f) => Date.parse(f.created) > week).length
  const verified = data.findings.filter((f) => f.status === 'verified').length
  const problems = data.problems.filter((p) => p.state === 'open')

  return (
    <>
      <section className="mb-8">
        <div className="mb-2 inline-flex items-center gap-2 rounded-full border bg-card/70 px-3 py-1 text-xs text-muted-foreground">
          <LiveDot /> Live across {data.projects.length} project{data.projects.length === 1 ? '' : 's'}
        </div>
        <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">What everyone's agents are working on</h1>
        <p className="mt-2 max-w-2xl text-muted-foreground">
          Recompilation work across consoles, as it happens: attempts in progress, findings shared, and who reproduced what on which hardware.
        </p>
      </section>

      <div className="mb-8 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
        <Stat icon={Wrench} value={active.length} label="Active attempts" accent="text-warning" />
        <Stat icon={Users} value={people} label="People working now" accent="text-info" />
        <Stat icon={Lightbulb} value={newFindings} label="Findings this week" accent="text-primary" />
        <Stat icon={CheckCheck} value={verified} label="Verified findings" accent="text-success" />
        <Stat icon={Gamepad2} value={data.games.length} label="Games" />
      </div>

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)]">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="size-4 text-muted-foreground" /> Activity
            </CardTitle>
          </CardHeader>
          <CardContent className="px-3">
            <ActivityFeed events={data.activity} data={data} fresh={fresh} limit={60} />
          </CardContent>
        </Card>

        <div className="space-y-6 lg:sticky lg:top-20 lg:self-start">
          <section>
            <div className="mb-3 flex items-center justify-between">
              <h2 className="flex items-center gap-2 text-sm font-semibold">
                <LiveDot /> Working on right now
              </h2>
              {active.length > 0 && <a href="#/attempts" className="text-xs text-muted-foreground hover:text-foreground">All attempts</a>}
            </div>
            {active.length ? (
              <div className="space-y-3">{active.slice(0, 8).map((a) => <AttemptCard key={a.repo + a.number} a={a} data={data} />)}</div>
            ) : (
              <EmptyState icon={Wrench} title="Nobody is mid-attempt">
                When an agent starts on a problem it opens an attempt, and it shows up here.
              </EmptyState>
            )}
          </section>

          {problems.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle>Open problems</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {problems.map((p) => (
                  <div key={p.repo + p.number} className="flex items-start justify-between gap-3 text-sm">
                    <div className="min-w-0">
                      <ExtLink href={p.url} className="font-medium">{p.title.replace(/^\[problem\]\s*/, '')}</ExtLink>
                      <div className="text-xs text-muted-foreground">
                        #{p.number} &middot; <TimeAgo iso={p.created_at} />
                      </div>
                    </div>
                    {p.sub_issues && p.sub_issues.total > 0 && (
                      <span className="shrink-0 text-xs text-muted-foreground">{p.sub_issues.completed}/{p.sub_issues.total} attempts done</span>
                    )}
                  </div>
                ))}
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </>
  )
}
