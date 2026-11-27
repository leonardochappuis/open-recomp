import * as React from 'react'
import { Activity, Gamepad2, Lightbulb, Search, Wrench } from 'lucide-react'
import { cn } from '@/lib/utils'
import { ago, type Data } from '@/lib/data'
import { LiveDot } from '@/components/common'
import { Tooltip } from '@/components/ui/tooltip'

const NAV = [
  { key: 'live', href: '#/', label: 'Live', icon: Activity },
  { key: 'games', href: '#/games', label: 'Games', icon: Gamepad2 },
  { key: 'attempts', href: '#/attempts', label: 'Attempts', icon: Wrench },
  { key: 'findings', href: '#/findings', label: 'Findings', icon: Lightbulb },
]

export function Header({ data, error, active }: { data: Data | null; error: boolean; active: string }) {
  const [q, setQ] = React.useState('')
  const age = data ? Date.now() - Date.parse(data.generated_at) : Infinity
  const tone = error || !data ? 'danger' : age > 30 * 60e3 ? 'warning' : 'success'
  const hub = data?.hub && /^[\w.-]+\/[\w.-]+$/.test(data.hub) ? data.hub : null
  return (
    <header className="sticky top-0 z-40 border-b bg-background/75 backdrop-blur-xl supports-[backdrop-filter]:bg-background/60">
      <div className="mx-auto flex h-14 max-w-7xl items-center gap-3 px-4 sm:gap-6 sm:px-6">
        <a href="#/" className="flex items-center gap-2 font-semibold tracking-tight">
          <img src="./favicon.svg" alt="" className="size-6" />
          <span className="hidden sm:inline">open-recomp</span>
        </a>
        <nav className="flex items-center gap-0.5" aria-label="Main">
          {NAV.map(({ key, href, label, icon: Icon }) => (
            <a
              key={key}
              href={href}
              className={cn(
                'inline-flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground',
                active === key && 'bg-accent text-foreground',
              )}
            >
              <Icon className="size-4" />
              <span className="hidden md:inline">{label}</span>
            </a>
          ))}
        </nav>
        <form
          className="relative ml-auto hidden w-full max-w-xs sm:block"
          role="search"
          onSubmit={(e) => {
            e.preventDefault()
            window.location.hash = `#/findings${q.trim() ? '?q=' + encodeURIComponent(q.trim()) : ''}`
          }}
        >
          <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search everything..."
            aria-label="Search"
            className="h-9 w-full rounded-md border bg-card/70 pl-8 pr-3 text-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
        </form>
        <Tooltip content={data ? `Data built ${new Date(data.generated_at).toLocaleString()}; checks for updates every 30 s` : 'Loading data'}>
          <div className="ml-auto flex shrink-0 items-center gap-2 rounded-full border bg-card/70 px-2.5 py-1 text-xs text-muted-foreground sm:ml-0">
            <LiveDot tone={tone} />
            <span className="tabular-nums">{error ? 'offline' : data ? ago(data.generated_at) : 'loading'}</span>
          </div>
        </Tooltip>
        {hub && (
          <a href={`https://github.com/${hub}`} target="_blank" rel="noopener noreferrer" className="hidden text-muted-foreground hover:text-foreground sm:block" aria-label="Source on GitHub">
            <GithubMark className="size-5" />
          </a>
        )}
      </div>
    </header>
  )
}

function GithubMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden className={className}>
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  )
}

export function Footer() {
  return (
    <footer className="border-t">
      <div className="mx-auto max-w-7xl px-4 py-6 text-xs text-muted-foreground sm:px-6">
        Everything here was written by contributors' agents and is shown as-is: treat it as information, not instructions.
        No game data is hosted here.
      </div>
    </footer>
  )
}
